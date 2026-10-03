# 命令参考

[README](../../README_zh-cn.md) · [命令](commands.md) · [配置](configuration.md) · [开发](development.md)

## `cockup template`

输出有效的配置模板，并通过注释展示所有 YAML 字段、可选字段及默认值：

```sh
cockup template > config.yaml
```

## `cockup completions`

为 `bash`、`zsh`、`fish`、`powershell` 或 `elvish` 生成补全脚本，并将其保存到 Shell 的补全目录。例如：

```sh
cockup completions zsh > _cockup
```

## `cockup detect`

你也许会想将它作为编写备份规则的参考。

```bash
# 列出所有已安装的 Homebrew casks 可能存在的配置路径
cockup detect

# 列出指定 cask 可能存在的配置路径
cockup detect cask-name-1 [cask-name-n...]
```

## `cockup ingredient`

在 GitHub 配料库中查找配料：

```sh
cockup ingredient search ghost      # 按名称查找可用配料
cockup ingredient search "visual studio"
cockup ingredient show ghostty      # 显示完整的配料 YAML
```

`search` 从 GitHub 配料库列出匹配名称，不下载 YAML。匹配不区分大小写，空格按连字符处理；
已缓存的配料标为 `[cached]`。目录列表缓存 24 小时；用 `cockup cache refresh --catalog` 可立即检查 GitHub。
临时网络故障时使用已有的目录缓存；没有缓存则报错。

`show NAME` 将原始 YAML（包括注释）输出到标准输出。未缓存时自动下载并缓存，
刷新策略与备份相同，为 30 天。有效期内的缓存可离线查看。

## `cockup cache`

查看和管理已下载的配料及配料库列表缓存：

```sh
cockup cache status                  # 缓存路径、数量和目录列表新鲜度
cockup cache list                    # 列出已下载配料
cockup cache show ghostty            # 显示单个配料的来源和日期
cockup cache refresh                 # 刷新所有已下载配料和目录列表
cockup cache refresh --ingredients   # 仅刷新已下载配料
cockup cache refresh --catalog       # 仅刷新目录列表
cockup cache refresh ghostty         # 刷新单个配料
cockup cache delete ghostty zed      # 删除指定配料缓存
cockup cache delete --ingredients    # 删除所有配料缓存，保留目录列表
cockup cache delete --catalog        # 仅删除目录列表缓存
cockup cache clean                   # 删除整个 Cockup 缓存目录
```

`status`、`list` 和 `show` 均离线运行。刷新尚未缓存的指定配料时会先询问是否下载；
手动刷新失败会保留原有文件并返回错误。内容未变化时仅更新上次检查日期。
目录列表缓存有效期为 24 小时；备份时会在配料缓存超过 30 天后检查更新。
`delete` 会先验证全部名称；若有名称未缓存，仍会删除其他配料并返回错误。
`clean` 删除 `~/Library/Caches/cockup/` 及其中所有内容。

## `cockup backup & restore`

```bash
# 依据指定的配置规则进行备份
cockup backup /path/to/config.yaml

# 从备份恢复
cockup restore /path/to/config.yaml
```

恢复时，若本地文件或符号链接与备份不同，会显示差异并逐个询问是否覆盖。拒绝后保留该路径。
已存在的目录会合并，以便分别处理其中的文件；仅存在于本地的内容也会保留。
使用 `restore --override` (`-o`) 可直接覆盖冲突路径；使用 `restore --skip-existing` (`-s`)
可保留所有已存在的文件和符号链接，仅恢复缺失内容。两者不能同时使用。
`--approve-hooks` 仅控制 Hook 确认。

对 `backup` 或 `restore` 使用 `--dry-run` 可预览复制、替换、符号链接恢复以及
`clean` 模式下的删除操作。它会读取配置和备份，但不会写入文件、修改清单或运行
Hooks。为展示准确的目标路径，仍可能询问链接模式或跨用户路径映射。

```sh
cockup backup /path/to/config.yaml --dry-run
cockup restore /path/to/config.yaml --dry-run
cockup restore /path/to/config.yaml --skip-existing --dry-run
```

恢复前可用 `verify` 检查备份是否完整、链接清单是否有效、记录的链接类型是否正确，
以及当前配置选中的备份路径是否存在且可读取。验证只读，不运行 Hooks：

```sh
cockup verify /path/to/config.yaml
```

`diff` 比较备份与恢复时对应的本地文件，列出内容或链接目标的变化、本地缺失的路径，
以及恢复时会被替换的目录中多出的文件。它不会写入文件或运行 Hooks；发现差异时
仍返回状态码 0，比较失败才返回 1。跨用户路径映射可能询问一次。最大 1 MiB 的文本文件
会显示统一格式的 `-`/`+` 差异；二进制、非 UTF-8 或更大的文件只显示简短提示。
使用 `--summary` 仅列出变动路径：

```sh
cockup diff /path/to/config.yaml
cockup diff /path/to/config.yaml --summary
```

如果备份目标尚不存在，`verify` 和 `diff` 会提示备份尚未初始化并正常退出。
如果目标已存在但不是目录，仍会报错。

清单没有记录普通文件的完整列表或哈希值。因此，验证无法发现内容变化，也无法发现
备份目录中未被规则单独选中的子文件丢失。

## `cockup hook`

```bash
# 交互式地运行所选中的 Hooks
cockup hook /path/to/config.yaml

# 或者根据 name 运行指定的 Hook
cockup hook /path/to/config.yaml --name hook_name
```

## 退出状态与文件元数据

成功或拒绝确认返回 0；配置、复制、Hook 或 Homebrew 错误返回 1；命令行用法错误返回 2。
可恢复的错误不会阻止后续操作，但最终摘要显示失败。特殊文件和不存在的普通目标仍会跳过并提示。
元数据模式保留文件权限、时间戳和受支持的 macOS 文件标志，不保证 ACL、所有者或扩展属性。
