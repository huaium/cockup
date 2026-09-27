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

# Back up with sample/CASE.yaml; user-remap simulates Alice only for Cockup.
sample-backup CASE="basic" *ARGS:
    #!/bin/sh
    set -eu
    cargo build --locked
    case {{ quote(CASE) }} in
        user-remap) env HOME="$(pwd -P)/sample/src/homes/alice" USER=alice ./target/debug/cockup backup sample/user-remap.yaml {{ ARGS }} ;;
        *) ./target/debug/cockup backup "sample/"{{ quote(CASE) }}".yaml" {{ ARGS }} ;;
    esac

# Restore with sample/CASE.yaml; user-remap simulates Bob only for Cockup.
sample-restore CASE="basic" *ARGS:
    #!/bin/sh
    set -eu
    cargo build --locked
    case {{ quote(CASE) }} in
        user-remap) env HOME="$(pwd -P)/sample/src/homes/bob" USER=bob ./target/debug/cockup restore sample/user-remap.yaml {{ ARGS }} ;;
        *) ./target/debug/cockup restore "sample/"{{ quote(CASE) }}".yaml" {{ ARGS }} ;;
    esac

# Run a named sample hook, or select interactively when NAME is omitted.
sample-hook NAME="":
    cargo run --locked -- hook sample/hooks.yaml --quiet {{ if NAME == "" { "" } else { "--name " + quote(NAME) } }}
