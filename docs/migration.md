# Rust migration (0.2.0)

## Compatibility

The YAML field names and CLI commands are retained, including `check` and `env`.
Use `-a`/`--approve-hooks` to approve hook execution without a confirmation prompt;
the former `-q`/`--quiet` and `-y`/`--yes` flags are no longer accepted.
Checks run only after successful
commands. Timeout is optional and applies independently to command and check;
the direct child is killed and reaped on timeout (not its entire process tree).

Configuration paths are resolved relative to each file. Included rules/hooks
precede local definitions; the root configuration controls destination and clean.
Destination and symlinks may be omitted from an included configuration to make
it an include-only ingredient; only rules is required. Standalone commands
require both destination and symlinks.
Includes now require objects such as `{file: child.yaml, wrap: imported}` rather
than strings. `wrap` prefixes the included rules' backup paths. Root symlinks and
metadata are defaults; include objects and individual rules can override them.
Includes are validated before actions, cycles fail, and hook
confirmation is shared across includes. A standalone hook or global pre-hook
runs in the root configuration directory; rule hooks and global post-hooks run
in the backup destination, matching the existing lifecycle.
Pre-backup and pre-restore hook failures stop their operation after the pre-hook
stage, before cleanup, copying, rule hooks, or post-hooks.

Failures now return status 1, including partial Homebrew discovery. Work continues
where possible; unusable destinations stop the operation. Usage errors return 2.
Declining confirmation returns 0. Missing literal files and special files are
warnings; unmatched patterns are errors. Clean operations refuse the filesystem
root, home directory, and source-containing destinations. Overlapping copies fail. Rule `to` and target paths must be relative and may not
contain `..`; invalid paths are rejected before clean mode removes anything.

Every standalone configuration requires `symlinks`: `reference`
copies links (including dangling links), `dereference` copies their targets, and
`prompt` asks once for links whose effective rule policy is prompt. The root policy
is the default and can be overridden by include objects and rules. Existing
configurations should add `symlinks: reference`.
Backup writes a versioned `.cockup-symlinks.json` with user/home identity and
link locations, target text, resolved targets, modes, target chains and relative
content paths. Restore uses recorded modes: reference restores only the link;
dereference restores target contents and recreates links. Prompt choices are
collected before cleanup and reused for copying and restore. Copied reference links keep their original
targets by adjusting relative paths; restore recreates the recorded link text.
Links into source or destination are allowed. Cleanup never follows symlinks when
deleting; files inside the cleaned destination are still removed normally.
Different-home restores ask once whether to map home prefixes to the current user
or keep the original paths, even with `--approve-hooks`. Blank/invalid input aborts before
restore writes. The JSON manifest is atomically replaced on successful backup;
failed/interrupted operations leave `.cockup-incomplete`, blocking restore until
backup succeeds. Completed copies are not rolled back. Missing manifests support
legacy reference-only restore without user remapping; corrupt manifests fail.
Directory targets are
replaced when updated; unrelated destination entries remain unless clean is true.
Regular-file permissions are always preserved. Metadata mode also preserves
file/link timestamps and macOS file flags; directory metadata, ownership, ACLs,
and extended attributes are not part of the contract.

## TDD and local validation

Tests exercise the compiled binary, temporary filesystem results, prompts, output,
exit status, and executable fixtures. CLI, invalid configuration, confirmation,
copying, hook execution, timeouts, discovery, overlap protection, and no-argument
help each went through observed failing tests before implementation. Additional
regression cases cover restore lifecycle, output controls, null values and dotfiles.
The Python reference suite passed all 165 tests before cutover.

A native Homebrew discovery smoke test succeeded. The local Apple Silicon release
build succeeds. Intel compilation requires the missing `x86_64-apple-darwin`
standard library; the workflow tests/builds on a native Intel runner. CI has not
been run from this local session.

## Performance observations

Measured locally on 2026-09-27: Python 3.14.7 versus an optimized Rust 1.98.1 build.
One warm-up followed by five measured runs per workload; table shows median wall
clock milliseconds, including process startup, with output redirected. Each backup
uses fresh clean destinations on the same filesystem and the same source fixtures.

| Workload | Python | Rust |
| --- | ---: | ---: |
| CLI help/startup | 61.37 | 2.81 |
| 1,000 files, 1 KiB each | 316.90 | 259.14 |
| One 64 MiB file | 76.84 | 3.56 |

These are local warm-cache observations, not throughput guarantees. In particular,
the large-file result likely benefits from APFS cloning/caching in the native copy
path and must not be extrapolated to other disks or filesystems. No timing assertion
is enforced in CI.

## Release preparation

The crate and executable are named `cockup`, version 0.2.0. Cargo search returned no
matching crate and its sparse-index entry returned HTTP 404 during implementation;
this does not reserve the name. Recheck availability/ownership before publishing.

The Rust workflow validates both macOS architectures on pushes and PRs. Version
tags produce architecture-specific archives and SHA256 checksums, publish through
the protected `crates-io` environment using its `CARGO_REGISTRY_TOKEN` secret, then
attach artifacts to a GitHub release. Configure the environment and required
reviewers before tagging. No release, tag, or crate was published during migration.

Old PyPI releases remain available. The Homebrew tap is a separate repository and
has not been updated. GitHub archives are unsigned and not notarized. The local `.venv`, Python caches, coverage reports, old distribution archives,
package metadata, and obsolete PyPI publishing environment file were removed
after migration at the user's request.
