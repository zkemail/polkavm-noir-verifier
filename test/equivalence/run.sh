#!/bin/bash
# Equivalence test: deploy a circuit's Solidity reference verifier (HonkVerifier.sol)
# to a local EVM devnet (anvil) and its generated Rust/PolkaVM verifier to a local
# PVM devnet (dev-node + eth-rpc), then run identical proof/corruption test vectors
# against both and assert matching accept/reject behavior.
#
# Usage:
#   ./run.sh <circuit_dir>
#   circuit_dir must contain target/HonkVerifier.sol, target/proof, target/public_inputs
#   (i.e. the output of `nargo execute && bb prove/write_vk/write_solidity_verifier`)
#
# Requires: anvil, forge, cast (foundry), cargo, polkatool, node/npx, python3, jq
# Requires ./bin/dev-node and ./bin/eth-rpc running - see bin/setup-dev-node.sh and
# the "Local devnets" section below for how to start them.
#
# Local devnets (start once, reused across circuit shapes):
#   ./bin/dev-node --dev --rpc-port 8001 &
#   ./bin/eth-rpc --dev --node-rpc-url=ws://127.0.0.1:8001 --rpc-port 8546 &
#   anvil --port 8547 &

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

CIRCUIT_DIR="${1:?Usage: ./run.sh <circuit_dir>}"
SOL_PATH="$CIRCUIT_DIR/target/HonkVerifier.sol"
PROOF_PATH="$CIRCUIT_DIR/target/proof"
PUB_INPUTS_PATH="$CIRCUIT_DIR/target/public_inputs"

for f in "$SOL_PATH" "$PROOF_PATH" "$PUB_INPUTS_PATH"; do
  if [ ! -f "$f" ]; then
    echo "Error: $f not found. Run 'nargo execute && bb prove/write_vk/write_solidity_verifier' first."
    exit 1
  fi
done

EVM_RPC="http://127.0.0.1:8547"
PVM_RPC="http://127.0.0.1:8546"

# Anvil's default account #0 - well-known, local-only, never used on any real network.
ANVIL_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
# dev-node's first pre-funded dev account (unlocked - dev-node signs for it itself).
PVM_DEV_ADDR="0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac"

echo "== [$CIRCUIT_DIR] Compiling & deploying Solidity reference to EVM devnet =="
mkdir -p "$SCRIPT_DIR/solidity-ref/src"
cp "$SOL_PATH" "$SCRIPT_DIR/solidity-ref/src/HonkVerifier.sol"
(cd "$SCRIPT_DIR/solidity-ref" && forge build --contracts src/HonkVerifier.sol) > /dev/null 2>&1
EVM_BYTECODE=$(jq -r '.bytecode.object' "$SCRIPT_DIR/solidity-ref/out/HonkVerifier.sol/HonkVerifier.json")
EVM_ADDRESS=$(cast send --rpc-url "$EVM_RPC" --private-key "$ANVIL_KEY" --create "$EVM_BYTECODE" --json | jq -r '.contractAddress')
echo "  EVM (Solidity reference) deployed: $EVM_ADDRESS"

echo "== [$CIRCUIT_DIR] Generating, building, and deploying Rust/PolkaVM verifier =="
OUT_DIR="$SCRIPT_DIR/.tmp-contract"
rm -rf "$OUT_DIR"
(cd "$REPO_ROOT/generator" && npx ts-node generate-verifier.ts honk --sol "$SOL_PATH" --out "$OUT_DIR" --build) > /dev/null
PVM_BYTECODE="0x$(xxd -p -c0 "$OUT_DIR/honk_verifier.polkavm")"
# No --gas-limit: pallet-revive's gas units are a different scale than EVM
# (see docs/kusama-grant/milestone-1/04_gas_optimization_benchmark_report.md,
# pending); estimation handles it correctly, a hardcoded EVM-sized limit doesn't.
PVM_ADDRESS=$(cast send --rpc-url "$PVM_RPC" --unlocked --from "$PVM_DEV_ADDR" --create "$PVM_BYTECODE" --json | jq -r '.contractAddress')
echo "  PVM (native Rust) deployed: $PVM_ADDRESS"

PROOF_HEX="0x$(xxd -p -c0 "$PROOF_PATH")"
PUB_HEX=$(xxd -p -c0 "$PUB_INPUTS_PATH")
PUB_ARRAY="["
for ((i = 0; i < ${#PUB_HEX}; i += 64)); do
  [ "$i" -gt 0 ] && PUB_ARRAY+=","
  PUB_ARRAY+="0x${PUB_HEX:$i:64}"
done
PUB_ARRAY+="]"

# call_both <label> <proof_hex> <pub_array>
# Calls verify() on both deployed contracts with identical calldata and asserts
# they agree: either both succeed, or both revert with the same 4-byte selector.
PASSED=0
TOTAL=0
call_both() {
  local label="$1" proof="$2" pubs="$3"
  TOTAL=$((TOTAL + 1))

  local evm_out evm_status pvm_out pvm_status
  evm_out=$(cast call "$EVM_ADDRESS" "verify(bytes,bytes32[])" "$proof" "$pubs" --rpc-url "$EVM_RPC" 2>&1) && evm_status=ok || evm_status=revert
  pvm_out=$(cast call "$PVM_ADDRESS" "verify(bytes,bytes32[])" "$proof" "$pubs" --rpc-url "$PVM_RPC" 2>&1) && pvm_status=ok || pvm_status=revert

  if [ "$evm_status" != "$pvm_status" ]; then
    echo "  $label: FAIL - EVM $evm_status, PVM $pvm_status (disagree)"
    echo "    EVM: $evm_out"
    echo "    PVM: $pvm_out"
    return
  fi

  if [ "$evm_status" = "ok" ]; then
    echo "  $label: PASS (both succeed)"
    PASSED=$((PASSED + 1))
    return
  fi

  # Both reverted - compare the 4-byte custom-error selector, not the raw
  # message text (EVM and PVM revert-message formatting differs, but the
  # selector is the part that's supposed to match - see the REVM-parity work
  # cited in 03_native_verifier_runtime.md).
  local evm_selector pvm_selector
  evm_selector=$(echo "$evm_out" | grep -oE '0x[0-9a-fA-F]{8}' | tail -1)
  pvm_selector=$(echo "$pvm_out" | grep -oE '0x[0-9a-fA-F]{8}' | tail -1)
  if [ "$evm_selector" = "$pvm_selector" ] && [ -n "$evm_selector" ]; then
    echo "  $label: PASS (both revert, selector $evm_selector)"
    PASSED=$((PASSED + 1))
  else
    echo "  $label: FAIL - both revert but selectors differ (EVM $evm_selector, PVM $pvm_selector)"
  fi
}

echo "== [$CIRCUIT_DIR] Running equivalence test vectors =="

call_both "Valid proof" "$PROOF_HEX" "$PUB_ARRAY"

BAD_PUB="[0x$(printf 'ff%.0s' $(seq 1 32))"
for ((i = 64; i < ${#PUB_HEX}; i += 64)); do
  BAD_PUB+=",0x${PUB_HEX:$i:64}"
done
BAD_PUB+="]"
call_both "Wrong public input" "$PROOF_HEX" "$BAD_PUB"

for OFFSET in 100 2000 4000; do
  CORRUPT="0x$(python3 -c "
d = bytes.fromhex('${PROOF_HEX:2}')
b = bytearray(d); b[$OFFSET] ^= 1
print(b.hex())
")"
  call_both "Corrupted proof (byte $OFFSET)" "$CORRUPT" "$PUB_ARRAY"
done

echo
echo "$PASSED/$TOTAL equivalence checks passed"
[ "$PASSED" -eq "$TOTAL" ] || exit 1
