#!/bin/bash
# Compiles all four legs' bytecode from the committed fixtures/*/target/HonkVerifier.sol
# sources into build/<leg>/<fixture>/. Run `npm install` first.
#
# Prerequisites not installed by npm (same ones the rest of this repo needs):
#   - cargo + polkatool (PVM native build, see generator/honk-verifier/static/README.md)
#   - solc >=0.8.21 (REVM/EVM leg; this project used 0.8.36 via Homebrew)
set -euo pipefail
cd "$(dirname "$0")"
ROOT=../../../..
FIXTURES=("noir-circuit" "zkemail")
OUT=build
rm -rf "$OUT"
mkdir -p "$OUT"

echo "=== PVM (native): generator build ==="
for f in "${FIXTURES[@]}"; do
  ( cd "$ROOT/generator" && node_modules/.bin/ts-node generate-verifier.ts honk \
      --sol "../fixtures/$f/target/HonkVerifier.sol" \
      --out "$(pwd)/../docs/kusama-grant/milestone-1/benchmarks/$OUT/pvm-native/$f" \
      --build )
done

echo "=== PVM (resolc) ==="
for f in "${FIXTURES[@]}"; do
  mkdir -p "$OUT/pvm-resolc/$f"
  node_modules/.bin/resolc --bin "$ROOT/fixtures/$f/target/HonkVerifier.sol" -o "$OUT/pvm-resolc/$f"
  # resolc names output files after the full input path; rename the main
  # contract's binary to a fixed name and drop the (unused) library stubs.
  main=$(find "$OUT/pvm-resolc/$f" -name "*_HonkVerifier.polkavm")
  mv "$main" "$OUT/pvm-resolc/$f/HonkVerifier.polkavm"
  find "$OUT/pvm-resolc/$f" -name "*.polkavm" ! -name "HonkVerifier.polkavm" -delete
done

echo "=== REVM / EVM (same bytecode, solc --optimize --optimize-runs 200) ==="
for f in "${FIXTURES[@]}"; do
  mkdir -p "$OUT/revm-evm/$f"
  solc --bin --optimize --optimize-runs 200 -o "$OUT/revm-evm/$f" --overwrite \
    "$ROOT/fixtures/$f/target/HonkVerifier.sol"
  xxd -r -p "$OUT/revm-evm/$f/HonkVerifier.bin" "$OUT/revm-evm/$f/HonkVerifier.raw"
done

echo
echo "Done. Bytecode sizes:"
find "$OUT" \( -name "*.polkavm" -o -name "*.raw" \) -exec ls -la {} \;
