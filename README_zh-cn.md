# cockup

[![CI](https://github.com/huaium/cockup/actions/workflows/rust.yml/badge.svg)](https://github.com/huaium/cockup/actions/workflows/rust.yml)

[English](README.md) | 中文

又一个用于备份各种配置文件的工具。

## 安装

Cockup 0.2.0 改用 Rust，支持 Apple Silicon 和 Intel macOS，无需 Python。

发布后可从 [GitHub Releases](https://github.com/huaium/cockup/releases) 下载对应架构的
压缩包，校验 SHA256 后解压，将 `cockup` 放入 `PATH` 中的目录。
也可在 crates.io 发布后安装：

```sh
cargo install cockup --locked
```

从源码构建：

```sh
cargo build --release --locked
./target/release/cockup --help
```

旧版 PyPI 包仍然可用，但不会安装 Rust 版本。现有 Homebrew tap 需要单独更新。

## 使用

### `cockup template`

输出有效的配置模板，并通过注释展示所有 YAML 字段、可选字段及默认值：

```sh
cockup template > config.yaml
```

### `cockup completions`

为 `bash`、`zsh`、`fish`、`powershell` 或 `elvish` 生成补全脚本，并将其保存到 Shell 的补全目录。例如：

```sh
cockup completions zsh > _cockup
```

### `cockup detect`

你也许会想将它作为编写备份规则的参考。

```bash
# 列出所有已安装的 Homebrew casks 可能存在的配置路径
cockup detect

# 列出指定 cask 可能存在的配置路径
cockup detect cask-name-1 [cask-name-n...]
```

### `cockup ingredient`

无需运行备份即可管理已下载的配料：

```sh
cockup ingredient update ghostty  # 即使缓存未过期，也立即检查 GitHub
cockup ingredient status           # 显示所有缓存的配料
cockup ingredient status ghostty   # 显示指定配料的来源和日期
cockup ingredient delete ghostty   # 删除指定配料的 YAML 和元数据缓存
cockup ingredient delete ghostty zed  # 一次删除多个配料缓存
cockup ingredient clean            # 删除整个 Cockup 缓存目录
```

`status` 离线且只读。`update` 请求失败时保留原有缓存并报告错误。收到 304 响应时仅更新上次检查日期。`delete` 只删除 `~/Library/Caches/cockup/ingredients/v1/` 中指定的缓存；`clean` 删除 `~/Library/Caches/cockup/` 及其全部内容。
`delete` 会先验证所有名称；如果其中一个配料未缓存，仍会删除其他配料，并返回错误。

### `cockup backup & restore`

```bash
# 依据指定的配置规则进行备份
cockup backup /path/to/config.yaml

# 从备份恢复
cockup restore /path/to/config.yaml
```

对 `backup` 或 `restore` 使用 `--dry-run` 可预览复制、替换、符号链接恢复以及
`clean` 模式下的删除操作。它会读取配置和备份，但不会写入文件、修改清单或运行
Hooks。为展示准确的目标路径，仍可能询问链接模式或跨用户路径映射。

```sh
cockup backup /path/to/config.yaml --dry-run
cockup restore /path/to/config.yaml --dry-run
```

恢复前可用 `verify` 检查备份是否完整、链接清单是否有效、记录的链接类型是否正确，
以及当前配置选中的备份路径是否存在且可读取。验证只读，不运行 Hooks：

```sh
cockup verify /path/to/config.yaml
```

清单没有记录普通文件的完整列表或哈希值。因此，验证无法发现内容变化，也无法发现
备份目录中未被规则单独选中的子文件丢失。

### `cockup hook`

```bash
# 交互式地运行所选中的 Hooks
cockup hook /path/to/config.yaml

# 或者根据 name 运行指定的 Hook
cockup hook /path/to/config.yaml --name hook_name
```

## 配置

创建一个遵循以下结构的 YAML 配置文件：

### 必要字段

```yaml
# 备份文件的存储位置
# 如果你打算使用相对路径，则请务必注意是相对于该配置文件的路径
destination: "/path/to/backup/directory"

# 必填：reference、dereference 或 prompt
symlinks: reference

# 备份规则列表
rules:
  - from: "/source/directory"
    targets: ["*.conf", "*.json"]
    to: "subdirectory"
```

单独运行配置文件时必须设置 `destination` 和 `symlinks`。省略它们的 YAML
只能通过 `include` 导入，且仅 `rules` 为必填字段。根配置的链接策略是默认值，
导入对象和单条规则可以覆盖它；恢复时使用清单记录的模式：

| 模式 | 行为 |
| --- | --- |
| `reference` | 复制符号链接本身，调整相对路径以保持原始目标，也支持失效链接。 |
| `dereference` | 保存链接指向的内容，并记录链接以便恢复。 |
| `prompt` | 遇到首个链接时询问一次，选择适用于所有链接：输入 `r`/`reference` 保留链接，或 `d`/`dereference` 复制目标。 |

该策略作用于匹配到的条目及递归复制目录时遇到的链接。`--quiet` 只跳过 Hook 确认，
不会跳过链接选择。无效输入会重新询问，输入流关闭时才会报错；无人值守运行应选择前两种模式。
链接选择在清理及目录替换之前完成。规划失败时，clean/prompt 备份会在删除备份数据前停止。

备份在根目录生成 `.cockup-symlinks.json`，请与备份内容一同保留。清单包含备份用户和主目录、
链接原位置、原始目标文本、解析后的目标、模式、目标链接链及内容的备份相对路径。
恢复时使用清单中的模式，不受 YAML 后续修改影响：

- `reference`：仅重建链接，不恢复目标内容。
- `dereference`：先恢复目标内容，再重建原始链接及记录的目标链接链。
- `prompt`：使用备份时记录的选择，不重复询问。

引用模式复制链接时会调整相对路径，使其仍指向原始目标；恢复时重建原始链接路径。
允许链接指向源目录或备份目录。`clean: true` 清理时只删除链接本身，不跟随链接；
位于备份目录内部的文件仍受正常清理规则影响。

恢复时，若路径属于不同备份用户的主目录，只询问一次：`c`/`current` 映射到当前用户主目录，
或 `o`/`original` 保留原主目录。选择统一作用于文件位置、目标内容和链接目标（也包括显式含旧用户名的相对链接）。
匹配主目录路径前缀，不替换任意用户名文本。`--quiet` 不跳过此询问，无效或缺失输入会在写入前终止恢复。

成功备份后原子替换清单。复制失败或中断会留下 `.cockup-incomplete`，再次成功备份前拒绝恢复，
避免使用不完整的数据；这不会回滚已经完成的复制。只有旧版 `reference` 备份允许无清单恢复（不支持用户路径映射）。
无效清单会在恢复前报错。清单及未完成标记路径为保留路径。

### 可选字段

```yaml
# 导入其他配置文件中的规则和 Hooks
include:
  - file: "./another_config.yaml"
    wrap: imported
    symlinks: dereference
    metadata: false

# 清洁模式，即是否先删除现有备份 (default: false)
clean: false

# 是否在备份时保留元数据 (default: true)
metadata: true

# 全局 Hooks
hooks:
  pre-backup:
    - name: "Setup"
      command: ["echo", "Starting backup"]
  post-backup:
    - name: "Cleanup"
      command: ["echo", "Backup complete"]
  pre-restore:
    - name: "Prepare"
      command: ["echo", "Starting restore"]
  post-restore:
    - name: "Finish"
      command: ["echo", "Restore complete"]
```

### 规则结构

每条规则都定义了程序所要备份的内容：

```yaml
- from: "/source/directory/in/pattern" # 允许使用通配符匹配
  targets:
    # 在 `from` 目录下的文件夹或文件
    - "pattern1" # 同样允许使用通配符匹配
    - "relative/path/to/file"
  to: "backup/subdirectory" # 在 `destination` 下的子文件夹
  on-start: # 规则层面的可选 Hooks
    - name: "Before Rule"
      command: ["echo", "Processing rule"]
  on-end:
    - name: "After Rule"
      command: ["echo", "Rule complete"]
```

请注意，当你在 `from` 中使用通配符时，为避免匹配多个目录导致的文件重复，cockup 会向上寻找第一个不存在通配符的安全目录，并将其设置为 `from`。而寻找过程中经过的包含通配符的路径，会拼接在每个 `target` 前。

例如，假设有如下的配置文件：

```yaml
- from: "/source/directory/in/prefix*/pattern"
  targets:
    - "relative/path/to/file"
  to: "backup/subdirectory"
```

我们最终会得到：

```yaml
- from: "/source/directory/in"
  targets:
    - "prefix*/pattern/relative/path/to/file"
  to: "backup/subdirectory"
```

### Hook 结构

Hooks 允许用户自定义运行命令。

默认情况下，如果你的配置文件包含任何 Hooks，程序会在执行命令前要求手动确认。使用 `--quiet` 或 `-q` 标志可以规避该行为。

如果你需要在一个特定的 Shell 里面运行命令，则请务必在检查安全性之后再使用诸如 `bash -c` 的命令来运行。

```yaml
- name: "Hook Name" # 必要：作为 Hook 的标识符
  command: ["cmd", "arg1"] # 必要：命令参数列表
  output: false # 可选：显示命令输出 (default: false)
  timeout: 10 # 可选：允许运行秒数
  env: # 可选: 用于该命令的环境变量
    ENV_1: 1
    ENV_2: 2
```

请注意，你无法直接使用类似 `$ENV_1` 的语法传递环境变量，但所启动的子进程能够访问这些变量。

一个典型的场景是备份 Homebrew bundle 列表，生成的文件将放置在 `destination` 指定的文件夹下：

```yaml
- name: "Brewfile Dumping"
  command: ["brew", "bundle", "dump", "--force", "--file", "Brewfile"]
  check: ["test", "-s", "Brewfile"]
  output: true
  timeout: 10
```

可选的 `check` 命令在 `command` 成功退出后执行，两者退出码都为 0 才算 Hook 成功。
检查命令使用相同的工作目录、`env` 和 `output`，`timeout` 分别作用于两个命令。
检查失败或超时会报告错误，同一阶段的后续 Hooks 仍会执行。
但只要任一备份前或恢复前 Hook 失败，操作就会在清理、复制、规则 Hooks 和后置 Hooks 之前停止。

### 配置导入

通过 `include` 对象导入其他配置文件的规则和 Hooks。导入的条目排在本地条目之前。每个对象必须且只能指定 `file` 或 `ingredient`。`file` 相对于声明它的配置文件解析；`ingredient` 指定[配料库](ingredients/README.md)中的名称。可选的 `wrap` 将导入规则的备份文件放进根目标目录下的指定子目录；嵌套导入的路径会逐层叠加。

根配置控制 `destination` 和 `clean`。被导入文件可以省略 `destination` 和 `symlinks`，成为不能单独运行的配料文件。根配置的 `symlinks` 和 `metadata` 是默认值：导入对象可以覆盖整个子树，单条规则可以再次覆盖。未指定的值从最近的导入对象继承，最终回退到根配置。被导入文件如有独立运行所需字段，其顶层设置仅在单独运行该文件时生效。恢复时，每条规则使用其有效的 `metadata` 值，而链接模式以备份清单为准。

```yaml
include:
  - file: "path_to/config_one.yaml"
    wrap: config-one
    symlinks: dereference
  - file: "path_to/config_two.yaml"
    metadata: false
  - ingredient: ghostty
    wrap: ghostty
```

Cockup 首次使用时从本仓库下载具名配料，缓存在 `~/Library/Caches/cockup/ingredients/v1/`，30 天后检查更新。更新失败时使用缓存；远端不存在的配料会报错。备份清单保存实际使用的配料 YAML，恢复和验证无需连接 GitHub。试运行可能下载配料，但不会写入缓存。

`wrap` 必须是非空相对路径，且不能包含 `..`。不再接受字符串形式的导入项。单条规则也可以设置 `symlinks` 或 `metadata`。

请访问 [sample](sample) 查看配置用例。

## 开发

使用原生 macOS 和支持 Rust 2024 的 Rust 工具链，无需虚拟环境。

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

## 从 Python 迁移

现有配置必须添加 `symlinks: reference` 才能保留原有链接复制行为。命令和其他 YAML 字段保持兼容，但 `include` 现在必须使用带 `file` 字段的对象。相对路径以各自配置文件所在目录为准，循环导入和无效配置会在执行前报错。
Hooks 只确认一次，`--quiet` 或 `-q` 同时作用于导入的 Hooks。

成功或拒绝确认返回 0；配置、复制、Hook 或 Homebrew 错误返回 1；命令行用法错误返回 2。
可恢复的错误不会阻止后续操作，但最终摘要显示失败。特殊文件和不存在的普通目标仍会跳过并提示。
元数据模式保留文件权限、时间戳和受支持的 macOS 文件标志，不保证 ACL、所有者或扩展属性。

参见 [迁移验证记录](docs/migration.md) 和 [样例](sample/README.md)。

## 许可证

请查阅 [LICENSE](./LICENSE)。
