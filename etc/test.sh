#!/bin/bash

cd "$(dirname "$0")"/.. || exit

cargo test --all --features mock_net

cd "$(dirname "$0")"/../payload || exit

cargo test --all