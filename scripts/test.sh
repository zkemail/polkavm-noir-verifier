#!/bin/bash
# Run verification tests against a deployed contract.
#
# Usage:
#   ./scripts/test.sh                                                              # fixtures + fixtures contract
#   ./scripts/test.sh /path/to/proof /path/to/public_inputs                        # custom proof, fixtures contract
#   ./scripts/test.sh /path/to/proof /path/to/public_inputs /path/to/contract_dir  # custom proof + contract
#
# Requires: cast (foundry)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RPC_URL="https://eth-rpc-testnet.polkadot.io/"

PROOF_PATH="${1:-$REPO_ROOT/fixtures/noir-circuit/target/proof}"
PUB_INPUTS_PATH="${2:-$REPO_ROOT/fixtures/noir-circuit/target/public_inputs}"
CONTRACT_DIR="${3:-$REPO_ROOT/contracts/honk-verifier}"

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

# Solidity ABI for `verify(bytes,bytes32[]) returns (bool)`:
#   success → 0x0000...0001 (32 bytes)
#   failure → REVERT with REVM-compatible custom-error selector:
#     0xed74ac0a = ProofLengthWrong()
#     0xfa066593 = PublicInputsLengthWrong()
#     0x9fc3a218 = SumcheckFailed()
#     0xa5d82e8a = ShpleminiFailed()
TRUE_BYTE="0x01"

# call_verify <proof_hex> <pub_array> → prints raw result hex (success) or
# error message containing "reverted" (failure).
call_verify() {
  cast call "$ADDRESS" "verify(bytes,bytes32[])" "$1" "$2" --rpc-url "$RPC_URL" 2>&1 || true
}

# is_revert <cast_output> → 0 (true) if cast reverted, 1 otherwise.
is_revert() {
  [[ "$1" == *"reverted"* ]]
}

# Test 1: Valid proof → bool true (`0x...01` 32-byte ABI)
RESULT=$(call_verify "$PROOF_HEX" "$PUB_ARRAY")
if [[ "$RESULT" == "$TRUE_BYTE" ]]; then
  echo "Test 1 - Valid proof:              PASS"; PASSED=$((PASSED+1))
else
  echo "Test 1 - Valid proof:              FAIL ($RESULT)"
fi

# Test 2: Wrong public input — must revert (SumcheckFailed, since corrupting pub
# inputs changes the Fiat-Shamir transcript so sumcheck round 0 fails).
BAD_ARRAY="[0x$(printf 'ff%.0s' $(seq 1 32))"
for ((i=64; i<${#PUB_HEX}; i+=64)); do
  BAD_ARRAY+=",0x${PUB_HEX:$i:64}"
done
BAD_ARRAY+="]"
RESULT=$(call_verify "$PROOF_HEX" "$BAD_ARRAY")
if is_revert "$RESULT"; then
  echo "Test 2 - Wrong public input:       PASS"; PASSED=$((PASSED+1))
else
  echo "Test 2 - Wrong public input:       FAIL ($RESULT)"
fi

# Test 3: Corrupted proof — must revert.
CORRUPT1="0x$(python3 -c "
d=bytes.fromhex('${PROOF_HEX:2}')
b=bytearray(d); b[2000]^=1; print(b.hex())
")"
RESULT=$(call_verify "$CORRUPT1" "$PUB_ARRAY")
if is_revert "$RESULT"; then
  echo "Test 3 - Corrupted univariate:     PASS"; PASSED=$((PASSED+1))
else
  echo "Test 3 - Corrupted univariate:     FAIL ($RESULT)"
fi

# Test 4: Corrupted evaluation — must revert.
CORRUPT2="0x$(python3 -c "
d=bytes.fromhex('${PROOF_HEX:2}')
b=bytearray(d); b[4000]^=1; print(b.hex())
")"
RESULT=$(call_verify "$CORRUPT2" "$PUB_ARRAY")
if is_revert "$RESULT"; then
  echo "Test 4 - Corrupted evaluation:     PASS"; PASSED=$((PASSED+1))
else
  echo "Test 4 - Corrupted evaluation:     FAIL ($RESULT)"
fi

# Test 5: Corrupted commitment — must revert.
CORRUPT3="0x$(python3 -c "
d=bytes.fromhex('${PROOF_HEX:2}')
b=bytearray(d); b[100]^=1; print(b.hex())
")"
RESULT=$(call_verify "$CORRUPT3" "$PUB_ARRAY")
if is_revert "$RESULT"; then
  echo "Test 5 - Corrupted commitment:     PASS"; PASSED=$((PASSED+1))
else
  echo "Test 5 - Corrupted commitment:     FAIL ($RESULT)"
fi

echo ""
echo "$PASSED/5 tests passed"
[ "$PASSED" -eq 5 ] || exit 1
