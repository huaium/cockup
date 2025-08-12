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