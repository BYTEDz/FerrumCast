#!/bin/bash
# run script for ferrumcast

export PATH=$PATH:/home/ahmed/.cargo/bin

# build engine
./scripts/build.sh || exit 1

# start orchestrator
echo "starting orchestrator..."
cd src/orchestrator
if [ ! -d "venv" ]; then
    python3 -m venv venv
    source venv/bin/activate
    pip install -r requirements.txt
else
    source venv/bin/activate
fi

python3 main.py
