# Sample

This directory contains fixtures for simple manual testing of cockup.

## Hook checks

From the project root, with `.venv` activated:

```bash
python -m cockup.main hook sample/config.yaml --name "Check created file" --quiet
python -m cockup.main hook sample/config.yaml --name "Check failure" --quiet
```

The first hook writes a file, checks its contents, and removes it. Expect
`Check passed` and `Completed 1/1 hook`.

The second hook runs successfully but its check deliberately exits with code 1.
Expect an error executing the check and `Completed 0/1 hook`. Hook failures are
reported in the output; the CLI currently still exits with code 0.

These hooks also run during sample backups; the intentional failure demonstrates
that subsequent hooks continue. Included configurations may still prompt for confirmation.
