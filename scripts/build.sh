#!/bin/bash
# build script for ferrumcast

export PATH=$PATH:/home/ahmed/.cargo/bin

echo "building engine..."
cargo build -p engine

echo "engine build done."
