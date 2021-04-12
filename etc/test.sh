#!/bin/bash

cd "$(dirname "$0")"/.. || exit

cargo test --all --features mock_net
