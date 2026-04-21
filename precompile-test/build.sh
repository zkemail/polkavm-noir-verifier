#!/bin/bash
set -e

cargo build --release

polkatool link --strip --output precompile_test.polkavm \
    target/riscv64emac-unknown-none-polkavm/release/precompile_test.elf

echo "Built: precompile_test.polkavm ($(wc -c < precompile_test.polkavm) bytes)"
