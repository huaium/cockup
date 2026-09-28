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

- CLI: clap subcommands `backup`, `restore`, `hook`, `list`, `template`, and `completions`.
  `template` prints a commented, valid YAML starter to stdout.
  `completions` prints generated Bash, Zsh, Fish, PowerShell, or Elvish scripts.
  Backup and restore accept `--dry-run` to preview changes without writes or hooks.
- Configuration: serde YAML models; each include object resolves against its own file.
  Included rules/hooks precede local ones; nested `wrap` paths prefix included
  backup paths. The root controls destination and clean; symlinks and metadata
  inherit through include objects and can be overridden per rule. Reject cycles
  and invalid configuration before operations. Prompt once for hook execution.
- File operations: explicit source/destination paths, glob layout preservation,
  clean/update modes, special files skipped. Required `symlinks` policy: referece
  links, dereference targets with cycle detection, or prompt once for all links. Root policy
  applies to included rules unless overridden. Discover links/choices before cleanup;
  keep copied links pointing at their original targets. Cleanup unlinks links
  without following them; destination contents remain subject to normal cleanup. `.cockup-symlinks.json` records
  modes, locations, targets/chains and user/home identity. Restore uses recorded
  modes: reference recreates only links; dereference restores contents then links.
  Cross-user restore asks once for current/original home mapping. Quiet only skips
  hook confirmation. Atomically replace manifests on success; incomplete backups
  block restore until a successful rerun. Preserve
  permissions; metadata mode also preserves file/link timestamps and macOS flags.
- Hooks: sequential command/check argument vectors (no implicit shell), inherited
  environment plus overrides, per-command timeout, configurable output. Checks
  run only after successful commands. Timeouts kill and reap the direct child.
  Complete the pre-hook stage, then abort backup or restore before file changes
  if any pre-hook failed.
- Discovery: bounded parallel `brew` subprocesses with auto-update disabled;
  parse cask JSON and print results in deterministic name order.
- Reporting: operational errors on stderr, red failed summaries, green successes.
  Exit 0 for success/declined confirmation, 1 for failures, 2 for CLI usage errors.
  Recoverable hook/copy failures accumulate while independent work continues.

## Testing

Use CLI-first TDD: one failing behavior test, then minimal implementation.
Agreed seams: CLI output/status/prompts and filesystem results. Use temporary
folders and executable fixtures for Homebrew and hooks; do not mock internals.
Never run sample backups against real user configuration directories. Shared
fixtures live in sample/src; configs use separate sample/dst/CASE destinations.
Use just sample-backup CASE, just sample-restore CASE, or just sample-hook NAME.
These recipes do not reset or mutate fixtures automatically. Intentional hook
failures are isolated in sample/hooks.yaml.

## Releases

CI tests both macOS architectures. Version tags build archives and checksums;
the protected `crates-io` environment supplies `CARGO_REGISTRY_TOKEN` for publishing.
Do not publish or create tags unless explicitly requested. See docs/migration.md.
