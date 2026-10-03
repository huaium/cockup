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
