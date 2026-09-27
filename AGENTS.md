# AGENTS.md

## Project

Cockup is a macOS configuration backup/restore CLI written in Rust 2024.
The binary is `cockup`; supported targets are `aarch64-apple-darwin` and
`x86_64-apple-darwin`. There is no public library API or Python runtime requirement.

## Development

- `cargo run --locked -- <command>`: run the CLI.
- `cargo test --locked`: integration tests through the compiled CLI.
- `cargo fmt --check` and `cargo clippy --locked --all-targets -- -D warnings`: checks.
- `cargo build --release --locked`: release binary in `target/release/cockup`.
- `just` lists recipes; `just run`, `just test`, and `just build` wrap Cargo.
- Keep Cargo.lock committed. Do not install or fetch dependencies on the user's
  behalf: explain additions, provide the exact command, and wait for the user.
- No Python installation or virtual environment activation is required.

## Architecture

- CLI: clap subcommands `backup`, `restore`, `hook`, and `list`.
- Configuration: serde YAML models; each include resolves against its own file.
  Included rules/hooks precede local ones; top-level settings win. Reject cycles
  and invalid configuration before operations. Prompt once for hook execution.
- File operations: explicit source/destination paths, glob layout preservation,
  clean/update modes, symlinks copied as links, special files skipped. Preserve
  permissions; metadata mode also preserves file/link timestamps and macOS flags.
- Hooks: sequential command/check argument vectors (no implicit shell), inherited
  environment plus overrides, per-command timeout, configurable output. Checks
  run only after successful commands. Timeouts kill and reap the direct child.
- Discovery: bounded parallel `brew` subprocesses with auto-update disabled;
  parse cask JSON and print results in deterministic name order.
- Reporting: operational errors on stderr, red failed summaries, green successes.
  Exit 0 for success/declined confirmation, 1 for failures, 2 for CLI usage errors.
  Recoverable hook/copy failures accumulate while independent work continues.

## Testing

Use CLI-first TDD: one failing behavior test, then minimal implementation.
Agreed seams: CLI output/status/prompts and filesystem results. Use temporary
folders and executable fixtures for Homebrew and hooks; do not mock internals.
Never run sample backups against real user configuration directories. Existing
sample backups intentionally contain a failing check and return status 1.

## Releases

CI tests both macOS architectures. Version tags build archives and checksums;
the protected `crates-io` environment supplies `CARGO_REGISTRY_TOKEN` for publishing.
Do not publish or create tags unless explicitly requested. See docs/migration.md.
