#!/bin/bash

cd "$(dirname "$0")"/..
set -ex

cargo test --all --features mock_net
