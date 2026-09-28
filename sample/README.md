# Samples

Use three commands from the project root:

```sh
just sample-backup                 # Basic files and includes
just sample-verify                 # Check the basic backup before restoring
just sample-restore
just sample-hook                   # Select a hook interactively
```

Choose a feature by its config name:

```sh
just sample-backup dereference
just sample-verify dereference
# Optionally edit a target file or remove a source link to observe restoration.
just sample-restore dereference

just sample-hook 'Check created file'
just sample-hook 'Check failure'    # Intentionally returns exit 1
```

| Config | What it tests |
| --- | --- |
| `basic` (default) | Files, globs, metadata, and rules from `basic-included.yaml` under `dst/basic/imported/`. |
| `reference` | Copy links while keeping their original targets; restore original link text. Target contents stay untouched. |
| `dereference` | Save target contents, restore them, and recreate source links. |
| `prompt` | Ask once for all links during backup; restore reuses the recorded decision. |
| `symlink-chain` | Preserve `alias/../file.txt` traversal through another symlink, without selecting the decoy file. |
| `user-remap` | Simulate Alice for backup and Bob for restore; ask once for `current` or `original` home. |

Each config writes to `dst/<config-name>` relative to `sample/`: for example,
`basic.yaml` uses `destination: dst/basic`. Input fixtures live in
`sample/src`, separate from backup output.
Fixtures are provided in the repository. Commands do not reset, mutate, or verify fixtures.

Every fixture under `src/` is used, either directly or as a symlink target:

| Source fixture | Used by |
| --- | --- |
| `config_file.txt` | `basic-included.yaml`, imported by `basic.yaml` |
| `system_log.txt`, `log_folder/` and its contents | `basic.yaml` |
| `file-link`, `directory-link`, `originals/` and its contents | `reference.yaml`, `dereference.yaml`, `prompt.yaml` |
| `symlink-chain/` and its contents | `symlink-chain.yaml`; `alias` points to shared `originals/dir/` |
| `homes/alice/config/`, `homes/alice/data/` and their contents | `user-remap.yaml`, directly and through its links |

The `homes/` and `homes/alice/` folders contain the user-remapping fixtures.
`homes/bob/`, if created by restore, is generated output rather than an input fixture.

Extra CLI arguments follow the config name, for example:

```sh
just sample-backup reference --quiet
just sample-restore reference --quiet
```

`--quiet` skips hook confirmation, not symlink choices or user-remapping questions.
The YAML mode spelling is `referece`; the config name is `reference`.

For user remapping, the recipes set `HOME` and `USER` only for Cockup, using
`sample/src/homes/alice` and `sample/src/homes/bob`. No real user accounts or home
directories are changed. Restore can overwrite sample files, so save any manual
fixture edits you want to keep.

Inspect `sample/dst/CONFIG/.cockup-symlinks.json` to see recorded link paths and modes.

Test a relative target containing a symlink followed by `..`:

```sh
just sample-backup symlink-chain
cat sample/dst/symlink-chain/files/link
cat sample/src/originals/file.txt
just sample-restore symlink-chain
```

The two `cat` commands should show identical contents, not the `DECOY` text in
`src/symlink-chain/file.txt`. The source link is `alias/../file.txt`, with `alias`
pointing to `../originals/dir`. Backup keeps this traversal intact; restore
recreates the original link text.
