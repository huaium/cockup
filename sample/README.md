# Sample

Fixtures for manual backup, restore, and hook tests. From the project root:

```sh
cargo run --locked -- hook sample/config.yaml --name "Check created file" --quiet
cargo run --locked -- hook sample/config.yaml --name "Check failure" --quiet
```

The first hook creates a file, checks its contents, and removes it. Expect
`Check passed`, `Completed 1/1 hook`, and exit status 0.

The second hook's check intentionally fails. Expect a red error summary and
exit status 1. Both hooks also run during sample backups: processing continues,
but the overall backup exits with status 1. Confirmation applies once to all
included hooks; `--quiet` skips it.

`just sample-backup`, `just sample-restore`, and `just sample-hook` are shortcuts.
Sample backup/restore commands modify the tracked fixtures; use a temporary copy
of this directory if you want to keep them unchanged.
