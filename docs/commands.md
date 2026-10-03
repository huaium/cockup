# Command reference

[README](../README.md) · [Commands](commands.md) · [Configuration](configuration.md) · [Development](development.md)

## `cockup template`

Print a valid configuration with commented examples of every YAML field, including
optional fields and their defaults:

```sh
cockup template > config.yaml
```

## `cockup completions`

Generate a completion script for `bash`, `zsh`, `fish`, `powershell`, or `elvish`
and save it in your shell's completion directory. For example:

```sh
cockup completions zsh > _cockup
```

## `cockup detect`

You may want to use it as a reference when writing your own backup rules.

```bash
# Detect potential config paths for all installed Homebrew casks
cockup detect

# Detect potential config paths for specified casks
cockup detect cask-name-1 [cask-name-n...]
```

## `cockup ingredient`

Find ingredients in the GitHub library:

```sh
cockup ingredient search ghost      # Find available ingredients by name
cockup ingredient search "visual studio"
cockup ingredient show ghostty      # Print the full ingredient YAML
```

`search` lists matching names from the GitHub library without downloading their
YAML. It matches names case-insensitively and treats spaces like hyphens; cached
ingredients are marked `[cached]`. The catalog is cached for 24 hours. On a
temporary network failure, search uses a previously cached catalog; without
one, it reports an error.

`show NAME` prints the original YAML, including comments, to stdout. It downloads
and caches missing ingredients automatically and uses the same 30-day refresh
policy as backup. Fresh cached ingredients can be shown offline.

## `cockup cache`

Inspect and manage downloaded ingredients and the cached library list:

```sh
cockup cache status                  # Cache path, counts, and catalog freshness
cockup cache list                    # Downloaded ingredient names
cockup cache show ghostty            # Source and dates for one ingredient
cockup cache refresh                 # Refresh all downloaded ingredients and catalog
cockup cache refresh --ingredients   # Refresh downloaded ingredients only
cockup cache refresh --catalog       # Refresh catalog only
cockup cache refresh ghostty         # Refresh one ingredient
cockup cache delete ghostty zed      # Delete named ingredient caches
cockup cache delete --ingredients    # Delete all ingredient caches, keeping the catalog
cockup cache delete --catalog        # Delete only the catalog cache
cockup cache clean                   # Remove the entire Cockup cache directory
```

`status`, `list`, and `show` are offline. Refreshing a named ingredient that is
not cached asks before downloading it. A failed manual refresh preserves existing
files and returns an error. An unchanged response updates only the check date.
The catalog refresh interval is 24 hours; downloaded ingredients are checked
after 30 days during backup. `delete` checks all names before removing files; if
one is missing, it still deletes the others and returns an error. `clean` removes
`~/Library/Caches/cockup/` and everything inside it.

## `cockup backup & restore`

```bash
# Backup files according to configuration
cockup backup /path/to/config.yaml

# Restore files from backup
cockup restore /path/to/config.yaml
```

Restore shows a diff and asks before replacing each conflicting local file or
symlink. Declining keeps that path. Existing directories are merged so choices
for individual files are respected; local-only entries remain in place.
Use `restore --override` (`-o`) to replace conflicting paths without prompts,
or `restore --skip-existing` (`-s`) to keep every existing file and symlink while
restoring missing ones. The flags cannot be combined. `--approve-hooks` only
controls hook confirmation.

Use `--dry-run` with `backup` or `restore` to preview copies, replacements,
symlink restoration, and clean-mode removals. It reads the configuration and
backup but does not write files, change the manifest, or run hooks. Symlink and
cross-user path choices may still be requested so the preview can show their
destinations.

```sh
cockup backup /path/to/config.yaml --dry-run
cockup restore /path/to/config.yaml --dry-run
cockup restore /path/to/config.yaml --skip-existing --dry-run
```

Use `verify` before restore to check that the backup is complete, its symlink
manifest is valid, recorded links have the expected type, and paths selected by
the current configuration are present and readable. Verification is read-only
and does not run hooks:

```sh
cockup verify /path/to/config.yaml
```

Use `diff` to compare the backup with the files it would restore. It reports
changed contents or link targets, paths missing locally, and extra files in
directories that restore would replace. It does not write files or run hooks;
differences return status 0, while comparison errors return status 1. Cross-user
home mapping may still prompt once. Text files up to 1 MiB show unified `-`/`+`
differences; binary, non-UTF-8, and larger files get a brief notice. Use
`--summary` for changed paths only:

```sh
cockup diff /path/to/config.yaml
cockup diff /path/to/config.yaml --summary
```

If no backup destination exists yet, `verify` and `diff` warn that the backup
is not initialized and exit successfully. An existing non-directory destination
is still an error.

The manifest does not inventory or hash ordinary files. Verification cannot
detect changed contents or a missing child inside a backed-up directory if that
child is not selected separately by a rule.

## `cockup hook`

```bash
# Run hooks interactively
cockup hook /path/to/config.yaml

# Or, run a specified hook by its name
cockup hook /path/to/config.yaml --name hook_name
```

## Exit status and file metadata

Exit codes are 0 for success or declined confirmation, 1 for configuration/copy/hook/
Homebrew failures, and 2 for CLI usage errors. Recoverable failures do not stop later
work, but the final summary reports failure. Special files and missing literal targets
are skipped with a warning. Metadata mode preserves file permissions, timestamps,
and supported macOS file flags; ACLs, ownership, and extended attributes are not guaranteed.
