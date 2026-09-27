default:
    @just --list

run *ARGS:
    cargo run --locked -- {{ ARGS }}

# Download the locked dependencies
sync:
    cargo fetch --locked

sync-all: sync

test *ARGS:
    cargo test --locked {{ ARGS }}

sample-backup *ARGS:
    cargo run --locked -- backup sample/config.yaml {{ ARGS }}

sample-restore *ARGS:
    cargo run --locked -- restore sample/config.yaml {{ ARGS }}

sample-hook NAME="":
    cargo run --locked -- hook sample/config.yaml {{ if NAME == "" { "" } else { "--name " + quote(NAME) } }}

build *ARGS:
    cargo build --release --locked {{ ARGS }}

check:
    cargo fmt --check
    cargo clippy --locked --all-targets -- -D warnings

clean:
    cargo clean

# Create and push a specific tag
tag VERSION:
    #!/usr/bin/env bash
    if [[ ! "{{ VERSION }}" =~ ^v.+ ]]; then
        echo "Version must start with v"
        exit 1
    fi

    read -p "Are you sure to create and push tag {{ VERSION }}? [y/N] " REPLY
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Operation cancelled"
        exit 0
    fi

    git tag {{ VERSION }}
    git push origin {{ VERSION }}

# Delete a specific tag
dtag VERSION:
    #!/usr/bin/env bash
    if [[ ! "{{ VERSION }}" =~ ^v.+ ]]; then
        echo "Version must start with v"
        exit 1
    fi

    read -p "Are you sure to delete and push tag {{ VERSION }}? [y/N] " REPLY
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Operation cancelled"
        exit 0
    fi

    git tag -d {{ VERSION }}
    git push origin --delete {{ VERSION }}
