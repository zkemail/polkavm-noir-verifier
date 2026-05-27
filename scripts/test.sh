#!/bin/bash
# Run verification tests against a deployed contract.
#
# Usage:
#   ./scripts/test.sh                                          # uses fixtures/noir-circuit/target
#   ./scripts/test.sh /path/to/proof /path/to/public_inputs    # custom paths
#
# Requires: cast (foundry)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONTRACT_DIR="$REPO_ROOT/contracts/honk-verifier"
RPC_URL="https://eth-rpc-testnet.polkadot.io/"

PROOF_PATH="${1:-$REPO_ROOT/fixtures/noir-circuit/target/proof}"
PUB_INPUTS_PATH="${2:-$REPO_ROOT/fixtures/noir-circuit/target/public_inputs}"

if ! command -v cast &> /dev/null; then
  echo "Error: 'cast' not found. Install foundry: https://getfoundry.sh"
  exit 1
fi

if [ ! -f "$CONTRACT_DIR/deployment.json" ]; then
  echo "Error: No deployment found at $CONTRACT_DIR/deployment.json"
  echo "Deploy first: cd contracts/honk-verifier && npx ts-node scripts/deploy.ts"
  exit 1
fi

if [ ! -f "$PROOF_PATH" ] || [ ! -f "$PUB_INPUTS_PATH" ]; then
  echo "Error: proof or public_inputs not found"
  exit 1
fi

ADDRESS=$(python3 -c "import json; print(json.load(open('$CONTRACT_DIR/deployment.json'))['address'])")
PROOF_HEX="0x$(xxd -p -c0 "$PROOF_PATH")"

# Build bytes32[] argument
PUB_HEX=$(xxd -p -c0 "$PUB_INPUTS_PATH")
NUM_INPUTS=$(( ${#PUB_HEX} / 64 ))
PUB_ARRAY="["
for ((i=0; i<${#PUB_HEX}; i+=64)); do
  [ $i -gt 0 ] && PUB_ARRAY+=","
  PUB_ARRAY+="0x${PUB_HEX:$i:64}"
done
PUB_ARRAY+="]"

echo "Contract: $ADDRESS"
echo "Proof: $(stat -f%z "$PROOF_PATH" 2>/dev/null || stat -c%s "$PROOF_PATH")B, Public inputs: $NUM_INPUTS"
echo ""

PASSED=0

# call_verify <proof_hex> <pub_array> → prints first return byte or "ERROR"
call_verify() {
  local result
  result=$(cast call "$ADDRESS" "verify(bytes,bytes32[])" "$1" "$2" --rpc-url "$RPC_URL" 2>&1) || true
  echo "$result"
}

# Test 1: Valid proof
RESULT=$(call_verify "$PROOF_HEX" "$PUB_ARRAY")
if [[ "$RESULT" == "0x01" ]]; then
  echo "Test 1 - Valid proof:              PASS"; PASSED=$((PASSED+1))
else
  echo "Test 1 - Valid proof:              FAIL ($RESULT)"
fi

# Test 2: Wrong public input — replace first input with 0xff..ff
BAD_ARRAY="[0x$(printf 'ff%.0s' $(seq 1 32))"
for ((i=64; i<${#PUB_HEX}; i+=64)); do
  BAD_ARRAY+=",0x${PUB_HEX:$i:64}"
done
BAD_ARRAY+="]"
RESULT=$(call_verify "$PROOF_HEX" "$BAD_ARRAY")
if [[ "$RESULT" != "0x01" ]]; then
  echo "Test 2 - Wrong public input:       PASS"; PASSED=$((PASSED+1))
else
  echo "Test 2 - Wrong public input:       FAIL ($RESULT)"
fi

# Test 3: Corrupted proof — flip one byte using python
CORRUPT1="0x$(python3 -c "
d=bytes.fromhex('${PROOF_HEX:2}')
b=bytearray(d); b[2000]^=1; print(b.hex())
")"
RESULT=$(call_verify "$CORRUPT1" "$PUB_ARRAY")
if [[ "$RESULT" != "0x01" ]]; then
  echo "Test 3 - Corrupted univariate:     PASS"; PASSED=$((PASSED+1))
else
  echo "Test 3 - Corrupted univariate:     FAIL ($RESULT)"
fi

# Test 4: Corrupted evaluation
CORRUPT2="0x$(python3 -c "
d=bytes.fromhex('${PROOF_HEX:2}')
b=bytearray(d); b[4000]^=1; print(b.hex())
")"
RESULT=$(call_verify "$CORRUPT2" "$PUB_ARRAY")
if [[ "$RESULT" != "0x01" ]]; then
  echo "Test 4 - Corrupted evaluation:     PASS"; PASSED=$((PASSED+1))
else
  echo "Test 4 - Corrupted evaluation:     FAIL ($RESULT)"
fi

# Test 5: Corrupted commitment
CORRUPT3="0x$(python3 -c "
d=bytes.fromhex('${PROOF_HEX:2}')
b=bytearray(d); b[100]^=1; print(b.hex())
")"
RESULT=$(call_verify "$CORRUPT3" "$PUB_ARRAY")
if [[ "$RESULT" != "0x01" ]]; then
  echo "Test 5 - Corrupted commitment:     PASS"; PASSED=$((PASSED+1))
else
  echo "Test 5 - Corrupted commitment:     FAIL ($RESULT)"
fi

echo ""
echo "$PASSED/5 tests passed"
[ "$PASSED" -eq 5 ] || exit 1
