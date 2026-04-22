#!/bin/bash
# Generate a PolkaVM UltraHonk verifier from a HonkVerifier.sol
#
# Usage:
#   ./scripts/generate.sh                                          # from fixtures/circuit/target
#   ./scripts/generate.sh path/to/HonkVerifier.sol                 # custom path
#   ./scripts/generate.sh path/to/HonkVerifier.sol output/dir      # custom output

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

SOL_PATH="${1:-$REPO_ROOT/fixtures/noir-circuit/target/HonkVerifier.sol}"
OUT_DIR="${2:-$REPO_ROOT/contracts/honk-verifier}"

if [ ! -f "$SOL_PATH" ]; then
  echo "Error: $SOL_PATH not found"
  echo "Run 'nargo execute && bb prove ... && bb write_solidity_verifier ...' in your circuit directory first."
  exit 1
fi

cd "$REPO_ROOT/generator"
npx ts-node generate-verifier.ts honk --sol "$SOL_PATH" --out "$OUT_DIR" --build
