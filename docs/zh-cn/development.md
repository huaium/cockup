# 开发

[README](../../README_zh-cn.md) · [命令](commands.md) · [配置](configuration.md) · [开发](development.md)

从源码构建：

```sh
cargo build --release --locked
./target/release/cockup --help
```

使用原生 macOS 和支持 Rust 2024 的 Rust 工具链。

```sh
cargo run --locked -- detect
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

`just` 显示所有命令；`just run`、`just test`、`just build` 和样例命令调用 Cargo。
测试通过 CLI 和临时文件验证行为，使用可执行脚本模拟 Homebrew 和 Hooks。
真实的 Homebrew 验证可运行 `cargo run --locked -- detect iterm2`。

端到端工作流程参见[样例](https://github.com/huaium/cockup/blob/main/sample/README.md)。
