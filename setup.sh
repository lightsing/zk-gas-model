#!/bin/bash
set -ex

PYTHON_VERSION="3.13.3"

if [ -d ".venv" ]; then
  echo ".venv already exists, skipping setup."
  exit 0
fi

if ! pyenv versions --bare | grep -Fxq "$PYTHON_VERSION"; then
  pyenv install "$PYTHON_VERSION"
fi

pyenv local "$PYTHON_VERSION"
eval "$(pyenv init -)"

python -V

python3 -m venv --copie .venv
source .venv/bin/activate
pip install --upgrade pip
pip install -r requirements.txt
