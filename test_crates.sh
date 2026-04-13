#!/usr/bin/env bash

set -euo pipefail

cargo test -p flexcan-teensy4 --target aarch64-apple-darwin
cargo test -p icm20649 --target aarch64-apple-darwin
