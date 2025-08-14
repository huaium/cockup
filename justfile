default: run

run *ARGS:
    python cockup/main.py {{ ARGS }}

install:
    pip install -e .

install-test:
    pip install -e ".[test]"

test *ARGS:
    pytest {{ ARGS }}

sample-backup:
    python cockup/main.py backup sample/config.yaml

sample-restore:
    python cockup/main.py restore sample/config.yaml

sample-hook:
    python cockup/main.py hook sample/config.yaml

sample-hooks:
    just sample-hook

build *ARGS:
    uv build {{ ARGS }}

publish *ARGS:
    #!/usr/bin/env bash
    read -p "Are you sure to publish? [y/N] " REPLY
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Operation cancelled"
        exit 0
    fi

    uv publish {{ ARGS }}

# Create and push a specific tag
tag version:
    #!/usr/bin/env bash
    if [[ ! "{{version}}" =~ ^v.+ ]]; then
        echo "Version must start with v"
        exit 1
    fi

    read -p "Are you sure to create and push tag {{version}}? [y/N] " REPLY
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Operation cancelled"
        exit 0
    fi
    
    git tag {{version}}
    git push origin {{version}}

# Delete a specific tag
dtag version:
    #!/usr/bin/env bash
    if [[ ! "{{version}}" =~ ^v.+ ]]; then
        echo "Version must start with v"
        exit 1
    fi

    read -p "Are you sure to delete and push tag {{version}}? [y/N] " REPLY
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Operation cancelled"
        exit 0
    fi
    
    git tag -d {{version}}
    git push origin --delete {{version}}