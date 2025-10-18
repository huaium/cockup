default: run

run *ARGS:
    python -m cockup.main {{ ARGS }}

install:
    uv sync

install-test:
    uv sync --all-extras

test *ARGS:
    pytest {{ ARGS }}

sample-backup *ARGS:
    python cockup/main.py backup sample/config.yaml {{ ARGS }}

sample-restore *ARGS:
    python cockup/main.py restore sample/config.yaml {{ ARGS }}

sample-hook NAME="":
    python cockup/main.py hook sample/config.yaml --name "{{ NAME }}"

build *ARGS:
    uv build {{ ARGS }}

clean:
    rm -rf dist/

clean-pycache:
    find . -type d -name "__pycache__" -exec rm -rf {} +

publish *ARGS:
    #!/usr/bin/env bash
    read -p "Are you sure to publish? [y/N] " REPLY
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Operation cancelled"
        exit 0
    fi

    uv publish {{ ARGS }}

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