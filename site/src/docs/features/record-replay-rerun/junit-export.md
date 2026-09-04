---
icon: material/xml
description: Exporting JUnit XML reports from recorded runs.
---

# JUnit export

<!-- md:version 0.9.144 -->

Nextest supports exporting a recorded run as a [JUnit/XUnit XML](https://llg.cubic.org/docs/junit/) report. This turns JUnit generation into a two-step process: record the run first, then export the report on demand.

Compared to [live JUnit output](../../machine-readable/junit.md), exporting from a recording has some advantages:

- **No configuration required at run time.** The `junit.path` setting does not need to be set for the export to work.
- **The report is written to a path you choose**, rather than to a location derived from the store directory.
- **Reports work for crashed runs.** If nextest or the machine crashed mid-run, the recording still contains all events seen so far, and the export synthesizes the report header from the run's metadata. Live JUnit output cannot provide this, because the report is only written when the run finishes.
- **Reports can be generated on other machines** via [portable recordings](portable-recordings.md), e.g., from a recording generated in CI.

## Prerequisites

To enable run recording, see [_Setting up run recording_](index.md#setting-up-run-recording).

## Exporting JUnit reports

To export the latest recording as a JUnit report:

```bash
cargo nextest store export-junit latest
```

By default, this prints the report to standard output. To instead write the report to a file, use the `-o`/`--output` option:

```bash
cargo nextest store export-junit latest -o junit.xml
```

The command also accepts a full run ID or a unique prefix, similar to other `store` commands. Reports can also be exported from a [portable recording](portable-recordings.md):

```bash
cargo nextest store export-junit my-run.zip
```

## Report contents

The exported report is identical to the report a live run with `junit.path` configured would have written, because both are produced by the same code from the same events.

The [JUnit configuration](../../machine-readable/junit.md#configuration) settings that shape the report—`store-success-output`, `store-failure-output`, `report-skipped`, and `flaky-fail-status`, including [per-test overrides](../../configuration/per-test-overrides.md)—are resolved at run time and stored within the recording. The export does not read repository configuration, so it produces the same report on any machine, even if the configuration has changed since the run.

### Report name

The report name is determined with the following precedence:

1. The `--report-name` option, if passed to `cargo nextest store export-junit`.
2. The `junit.report-name` configuration setting resolved at the time the run was recorded.
3. The default, `"nextest-run"`.

### Incomplete runs

If a run was interrupted before completion (for example, nextest crashed mid-run), the export prints a warning and proceeds. The report contains all tests that finished before the interruption, and the report header (run ID, start time, and elapsed time) is synthesized from the run's recorded metadata.

### Older recordings

Recordings made by older nextest versions can be exported, with some limitations:

- Recordings made before store format 2.2 do not contain setup script output storage settings, so setup script output is omitted from the report.
- Recordings made before nextest recorded resolved JUnit settings fall back to the default report name.
- Recordings made before the JUnit settings were decoupled from `junit.path` carry all-false output storage settings if `junit.path` was not configured at run time. Reports exported from such recordings contain no `<system-out>`/`<system-err>` content.

## Options and arguments

### `cargo nextest store export-junit`

=== "Summarized output"

    The output of `cargo nextest store export-junit -h`:

    === "Colorized"

        ```bash exec="true" result="ansi"
        CLICOLOR_FORCE=1 cargo nextest store export-junit -h | ../scripts/strip-hyperlinks.sh
        ```

    === "Plaintext"

        ```bash exec="true" result="text"
        cargo nextest store export-junit -h | ../scripts/strip-ansi.sh | ../scripts/strip-hyperlinks.sh
        ```

=== "Full output"

    The output of `cargo nextest store export-junit --help`:

    === "Colorized"

        ```bash exec="true" result="ansi"
        CLICOLOR_FORCE=1 cargo nextest store export-junit --help | ../scripts/strip-hyperlinks.sh
        ```

    === "Plaintext"

        ```bash exec="true" result="text"
        cargo nextest store export-junit --help | ../scripts/strip-ansi.sh | ../scripts/strip-hyperlinks.sh
        ```
