# Configuration reference

[README](../README.md) · [Commands](commands.md) · [Configuration](configuration.md) · [Development](development.md)

Create a YAML configuration file with the following structure:

## Required Fields

```yaml
# Where backups are stored
# If you use relative path, it will be relative to the config file's directory
destination: "/path/to/backup/directory"

# Required: reference, dereference, or prompt
symlinks: reference

# List of backup rules
rules:
  - from: "/source/directory"
    targets: ["*.conf", "*.json"]
    to: "subdirectory"
```

`destination` and `symlinks` are required when running a configuration directly.
Omit them to make the YAML an include-only ingredient; only `rules` is required
in an ingredient.
The root value is the default; include objects and rules can override it:

| Mode | Behavior |
| --- | --- |
| `reference` | Copy the link itself, preserving its target meaning by adjusting relative paths, including dangling links. |
| `dereference` | Save target contents and record the link for restoration. |
| `prompt` | Ask once during backup: `r`/`reference` or `d`/`dereference`; apply the decision to all links. |

The policy applies to matched entries and symlinks encountered while recursively
copying directories. In `prompt` mode, `--approve-hooks` only suppresses hook confirmation;
it does not answer symlink questions. Invalid answers are prompted again; closed
input reports an error. Use `reference` or `dereference` for unattended runs. Dereferencing broken
links, cycles, or overlapping source/destination paths reports an error. Link
choices are collected before cleanup or directory replacement; planning failures
abort clean/prompt backups before deleting backup data.

Backup writes `.cockup-symlinks.json` at the backup root. Keep it with the backup.
It records the backup user/home, original link locations, exact targets, resolved
targets, handling modes, target chains, and relative paths to backed-up contents.
Restore follows the recorded modes, regardless of the current YAML mode:

- `reference`: recreate the link only; leave target contents untouched.
- `dereference`: restore target contents, then recreate the original link and any
  recorded links in its target chain.
- `prompt`: reuse the choices recorded during backup; do not ask for them again.

Reference copies keep pointing to their original targets, adjusting relative link
paths for the backup location. Restore recreates the original link text. Links
pointing into the source or destination are allowed. With `clean: true`, cleanup
removes links without following them; files inside the cleaned destination are
still subject to normal cleanup.

When restoring paths belonging to a different backup user/home, Cockup asks once:
`c`/`current` maps them to the current home; `o`/`original` retains the old home.
The choice applies to file locations, restored target contents, and link targets,
including relative links that explicitly name the old user. It matches home-path
prefixes, not arbitrary username text. `--approve-hooks` does not skip this question.
Invalid or missing input aborts before restoration writes.

Every backup rebuilds the manifest from current sources and configuration, including
when `clean: false`. An invalid or obsolete existing manifest requires confirmation
before hooks or backup file changes; declining cancels the backup. Valid manifests
require no confirmation. Dry-run previews rebuilding without prompting or writing.
`--approve-hooks` does not approve rebuilding the manifest.

The manifest is replaced atomically after a successful backup. A failed or
interrupted copy leaves `.cockup-incomplete`; restore refuses that backup until a
successful backup clears the marker. This detects incomplete backups, but does
not roll back copies already made. Missing manifests are accepted only for legacy
`reference` restores (without user remapping); malformed manifests fail before
restoration. The manifest and incomplete-marker paths are reserved.

## Optional Fields

```yaml
# Include rules and hooks from other config files
include:
  - file: "./another_config.yaml"
    wrap: imported
    symlinks: dereference
    metadata: false

# Clean mode, whether to remove existing backup folder (default: false)
clean: false

# Whether to preserve metadata when backing up (default: true)
metadata: true

# Global hooks
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

## Rule Structure

Each rule defines what to backup:

```yaml
- from: "/source/directory/in/pattern" # Wildcards are supported
  targets:
    # Folders or files under `from`
    - "pattern1" # Wildcards are also supported
    - "relative/path/to/file"
  to: "backup/subdirectory" # A folder under `destination`
  on-start: # Optional rule-level hooks
    - name: "Before Rule"
      command: ["echo", "Processing rule"]
  on-end:
    - name: "After Rule"
      command: ["echo", "Rule complete"]
```

Please be aware that when using wildcards in `from`, to prevent duplicate files caused by matching multiple directories, cockup will locate the first safe directory without wildcards and set it as `from`. The paths containing wildcards encountered during the search process will be prepended to each `target`.

For example, if you have:

```yaml
- from: "/source/directory/in/prefix*/pattern"
  targets:
    - "relative/path/to/file"
  to: "backup/subdirectory"
```

We will finally get:

```yaml
- from: "/source/directory/in"
  targets:
    - "prefix*/pattern/relative/path/to/file"
  to: "backup/subdirectory"
```

## Hook Structure

Hooks support custom commands.

By default, you will be prompted to confirm if your configuration file contains any hooks. Use the flag `--approve-hooks` or `-a` to suppress it.

If you want to run them within a specified shell, use commands like `bash -c` after ensuring your commands are safe.

```yaml
- name: "Hook Name" # Required: Hook identifier
  command: ["cmd", "arg1"] # Required: Command args list
  output: false # Optional: Print output (default: false)
  timeout: 10 # Optional: Timeout in seconds
  env: # Optional: environment variables used for the command
    ENV_1: 1
    ENV_2: 2
```

Please note that you cannot pass environment variables directly using syntax like `$ENV_1`, but the subprocess you launch can access those variables.

For example, you may want to use it to dump Homebrew bundle into a file and place it under the folder defined by `destination`:

```yaml
- name: "Brewfile Dumping"
  command: ["brew", "bundle", "dump", "--force", "--file", "Brewfile"]
  check: ["test", "-s", "Brewfile"]
  output: true
  timeout: 10
```

An optional `check` command runs after `command` exits successfully. Both must exit
with code 0 for the hook to count as successful. The check uses the same working
directory, `env`, and `output`; `timeout` applies separately to each command.
A failed or timed-out check is reported, and subsequent hooks in that stage still run.
If any pre-backup or pre-restore hook fails, the operation stops before cleanup,
copying, rule hooks, or post-hooks.

## Include

Use `include` objects to import rules and hooks from other configuration files. Included entries run before local entries. Each object specifies exactly one of `file` or `ingredient`. `file` is resolved relative to the configuration that declares it; `ingredient` names an entry in the [ingredient library](https://github.com/huaium/cockup/blob/main/ingredients/README.md). Optional `wrap` places all imported backup files under that path within the root destination; nested wrappers compose.

The root configuration controls `destination` and `clean`. An included file may omit `destination` and `symlinks` to become an ingredient that cannot run independently. Root `symlinks` and `metadata` are defaults: an include object can override them for its subtree, and an individual rule can override them again. Omitted values inherit from the nearest include, then the root. An included file's own top-level settings apply when it has both required standalone fields and is run directly, but its rules use the inherited settings when included. Restore uses each rule's effective `metadata` value and the symlink modes recorded by backup.

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

`wrap` must be a non-empty relative path without `..`. Rules can also set `symlinks` or `metadata` directly. Cockup downloads named ingredients from this repository on first use and caches them under `~/Library/Caches/cockup/ingredients/v1/`. It checks for updates after 30 days, using the cached copy if a refresh fails; a missing ingredient is an error. Backup records the exact ingredient YAML in its manifest, so restore and verify use the backed-up rules without contacting GitHub. Dry-run can fetch an ingredient but does not write to the cache.

Refer to [sample](https://github.com/huaium/cockup/tree/main/sample) to view a configuration demo.
