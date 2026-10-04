# Development

[README](../README.md) · [Commands](commands.md) · [Configuration](configuration.md) · [Development](development.md)

Build from source:

```sh
cargo build --release --locked
./target/release/cockup --help
```

Use native macOS and a Rust toolchain supporting Rust 2024.

```sh
cargo run --locked -- detect
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

`just` lists recipes; `just run`, `just test`, `just build`, and sample recipes wrap Cargo.
Tests exercise the CLI using temporary files and executable Homebrew/hook fixtures.
For a real Homebrew smoke test, run `cargo run --locked -- detect iterm2`.

See the [samples](https://github.com/huaium/cockup/blob/main/sample/README.md) for end-to-end workflows.

## Homebrew releases

After a tagged GitHub release succeeds, `update-tap` updates both architecture URLs
and SHA256 checksums in `huaium/homebrew-tap`'s `Formula/cockup.rb`. It verifies the
published archives, installs and tests the formula on the runner, then commits and
pushes to the tap's `main` branch. Repeating the job is safe; version downgrades are
rejected. The tap's existing CI checks the resulting push.

One-time setup: create a fine-grained personal access token with resource owner
`huaium`, repository access limited to `homebrew-tap`, and **Contents: read and write**.
In `huaium/cockup` → Settings → Secrets and variables → Actions, add it as the
repository secret `HOMEBREW_TAP_TOKEN`. The default `GITHUB_TOKEN` cannot write to
another repository. Keep the token out of source files and renew it before expiry.
If tap branch protection requires pull requests, use a PR-based update workflow
instead of granting a bypass.

Run updater checks without installing anything:

```sh
ruby scripts/test_update_homebrew.rb
```
