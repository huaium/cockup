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

## Homebrew 发布

标签对应的 GitHub Release 成功发布后，`update-tap` 会更新 `huaium/homebrew-tap`
中 `Formula/cockup.rb` 的两种架构下载地址和 SHA256。它先验证发布的压缩包，
在运行器上安装并测试公式，再提交并推送到 Tap 的 `main` 分支。
重复执行不会产生无效提交，版本降级会被拒绝。Tap 现有的 CI 会检查此次推送。

首次配置：创建细粒度个人访问令牌，资源所有者为 `huaium`，仅授权 `homebrew-tap`
仓库，并赋予 **Contents: read and write** 权限。在 `huaium/cockup` 的
Settings → Secrets and variables → Actions 中，将令牌保存为仓库 Secret
`HOMEBREW_TAP_TOKEN`。默认 `GITHUB_TOKEN` 不能写入其他仓库。
不要将令牌放入源码，并在过期前更新。若 Tap 分支保护要求 PR，改用 PR 更新流程。

无需安装软件即可运行更新器检查：

```sh
ruby scripts/test_update_homebrew.rb
```
