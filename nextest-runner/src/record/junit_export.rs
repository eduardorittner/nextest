// Copyright (c) The nextest Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! JUnit export from recorded test runs.
//!
//! This module converts recorded events back into live test events via
//! [`ReplayContext`], and feeds them to the same [`JunitReportBuilder`] that
//! the live JUnit aggregator uses. As a result, an exported report is
//! identical to the report a live run with `junit.path` configured would have
//! written.

use crate::{
    errors::{DisplayErrorChain, JunitExportError, RecordReadError},
    list::TestList,
    output_spec::RecordingSpec,
    record::{
        reader::StoreReader,
        replay::{LoadOutput, ReplayContext},
        store::RecordedRunInfo,
        summary::{OutputEventKind, RecordOpts, TestEventKindSummary, TestEventSummary},
    },
    reporter::JunitReportBuilder,
};
use quick_junit::Report;
use std::time::Duration;
use tracing::warn;

/// The default JUnit report name, used when the recording does not carry one.
///
/// This matches the `junit.report-name` default in the default profile
/// configuration.
pub const DEFAULT_JUNIT_REPORT_NAME: &str = "nextest-run";

/// Options for exporting a JUnit report from a recording.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct JunitExportOpts {
    /// Overrides the report name.
    ///
    /// Precedence: this override, then the name recorded in the archive, then
    /// [`DEFAULT_JUNIT_REPORT_NAME`].
    pub report_name_override: Option<String>,
}

impl JunitExportOpts {
    /// Creates a new `JunitExportOpts` with default settings.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Exports a JUnit report from a recorded run.
///
/// The test list must be reconstructed from the archived metadata via
/// [`TestList::from_summary`], and `record_opts` must be the recorded options
/// read from the same archive.
///
/// Events that fail to convert produce warnings and are skipped, the same as
/// replay. If the event log has no `RunFinished` event (an incomplete run),
/// the report trailer is synthesized from `run_info` and the elapsed time of
/// the last recorded event.
pub fn export_junit_report(
    test_list: &TestList<'_>,
    record_opts: &RecordOpts,
    store_reader: &mut dyn StoreReader,
    events: impl Iterator<Item = Result<TestEventSummary<RecordingSpec>, RecordReadError>>,
    opts: JunitExportOpts,
    run_info: &RecordedRunInfo,
) -> Result<Report, JunitExportError> {
    let report_name = resolve_report_name(opts.report_name_override, record_opts);

    let mut replay_cx = ReplayContext::new(test_list);
    for test in test_list.iter_tests() {
        replay_cx.register_test(test.id().to_owned());
    }

    let mut builder = JunitReportBuilder::new(test_list.mode(), report_name);
    let mut last_elapsed = Duration::ZERO;

    for event_result in events {
        let event_summary = event_result.map_err(JunitExportError::ReadError)?;
        last_elapsed = event_summary.elapsed;

        let load_output = junit_load_output(&event_summary.kind);
        match replay_cx.convert_event(&event_summary, store_reader, load_output) {
            Ok(event) => {
                builder
                    .write_event(event)
                    .map_err(JunitExportError::BuildReport)?;
            }
            Err(error) => {
                // Warn about conversion errors, but continue, the same as
                // replay.
                warn!(
                    "error converting recorded event: {}",
                    DisplayErrorChain::new(error)
                );
            }
        }
    }

    let report = match builder.take_report() {
        Some(report) => report,
        None => {
            // The event log has no RunFinished event: the run is incomplete.
            // Synthesize the trailer from the run metadata recorded at run
            // start, with the last event's elapsed time as the run duration.
            // This makes JUnit reports available for crashed runs, which live
            // JUnit cannot provide.
            builder.build_report_with_trailer(run_info.run_id, run_info.started_at, last_elapsed)
        }
    };

    Ok(report)
}

/// Resolves the report name: CLI override, then the recorded value, then the
/// default.
fn resolve_report_name(report_name_override: Option<String>, record_opts: &RecordOpts) -> String {
    report_name_override
        .or_else(|| {
            record_opts
                .junit
                .as_ref()
                .map(|junit| junit.report_name.clone())
        })
        .unwrap_or_else(|| DEFAULT_JUNIT_REPORT_NAME.to_owned())
}

/// Decides whether to load output from the archive for JUnit export.
///
/// JUnit needs materialized output in two cases: extracting the failure
/// message and description from failed units (independent of the store flags),
/// and filling in `<system-out>`/`<system-err>` when the applicable store flag
/// is set. This decider conservatively loads output for every event the report
/// builder consumes. Typical recorded outputs are small, so the cost is low.
///
/// This is intentionally separate from the display-driven
/// [`OutputLoadDecider`](crate::reporter::OutputLoadDecider) used by replay.
fn junit_load_output(kind: &TestEventKindSummary<RecordingSpec>) -> LoadOutput {
    match kind {
        TestEventKindSummary::Output(kind) => match kind {
            // The report builder reads test and setup script outputs.
            OutputEventKind::SetupScriptFinished { .. } | OutputEventKind::TestFinished { .. } => {
                LoadOutput::Load
            }
            // Retries are reported from the run statuses inside TestFinished;
            // the report builder ignores this event.
            OutputEventKind::TestAttemptFailedWillRetry { .. } => LoadOutput::Skip,
        },
        // Core events have no output.
        TestEventKindSummary::Core(_) => LoadOutput::Skip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{record::summary::RecordedJunitOpts, run_mode::NextestRunMode};

    #[test]
    fn report_name_precedence() {
        let recorded = RecordOpts::new(
            NextestRunMode::Test,
            Some(RecordedJunitOpts::new("recorded-name".to_owned())),
        );
        let not_recorded = RecordOpts::new(NextestRunMode::Test, None);

        // The CLI override wins over the recorded name.
        assert_eq!(
            resolve_report_name(Some("cli-name".to_owned()), &recorded),
            "cli-name"
        );
        // The recorded name wins over the default.
        assert_eq!(resolve_report_name(None, &recorded), "recorded-name");
        // The CLI override wins over the default.
        assert_eq!(
            resolve_report_name(Some("cli-name".to_owned()), &not_recorded),
            "cli-name"
        );
        // Recordings without a JUnit snapshot (store format 2.1 and earlier)
        // fall back to the default.
        assert_eq!(
            resolve_report_name(None, &not_recorded),
            DEFAULT_JUNIT_REPORT_NAME
        );
    }
}
