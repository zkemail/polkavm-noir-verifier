// Deploy + verify benchmark: sends a real contract-creation transaction and a
// real verify(bytes,bytes32[]) transaction, records gas used and fee paid.
// Bytecode is read from disk (not passed as a shell argument) to avoid
// argv-length limits on large payloads.
//
// Env: RPC_URL, PK, EXPECTED (burner addr guard), BYTECODE, PROOF, PUBLIC_INPUTS,
//   ADDRESS (skip deploy, verify against an existing contract), OUT, LABEL
// Optional: GAS_LIMIT (default 60000000; needed on chains where gas estimation
//   rejects contract-creation transactions, e.g. Paseo)
import { readFileSync, writeFileSync } from "fs";
import { ethers } from "ethers";

const { RPC_URL, PK, EXPECTED, BYTECODE, PROOF, PUBLIC_INPUTS, ADDRESS, OUT, LABEL, GAS_LIMIT } =
  process.env;
if (!RPC_URL || !PK || !EXPECTED || !PROOF || !PUBLIC_INPUTS || !OUT) {
  throw new Error("missing required env (RPC_URL, PK, EXPECTED, PROOF, PUBLIC_INPUTS, OUT)");
}

const provider = new ethers.JsonRpcProvider(RPC_URL);
const wallet = new ethers.Wallet(PK, provider);

if (wallet.address.toLowerCase() !== EXPECTED.toLowerCase()) {
  throw new Error(`signer ${wallet.address} != expected burner ${EXPECTED}; aborting`);
}

const gasLimit = GAS_LIMIT ? BigInt(GAS_LIMIT) : undefined;
const results = [];

// Poll for the receipt directly instead of tx.wait(), which throws on
// status=0 - pallet-revive reports status=0 on some genuinely-successful
// calls (multi-byte, non-32-byte-padded return data), so a thrown exception
// there would otherwise discard real gas data.
async function waitForReceipt(hash) {
  for (;;) {
    const rcpt = await provider.getTransactionReceipt(hash);
    if (rcpt) return rcpt;
    await new Promise((r) => setTimeout(r, 2000));
  }
}

async function record(name, sendFn, { knownStatusZeroIsSuccess = false } = {}) {
  const tx = await sendFn();
  console.log(`${name} tx: ${tx.hash}`);
  const rcpt = await waitForReceipt(tx.hash);
  const gasPrice = rcpt.gasPrice ?? tx.gasPrice ?? 0n;
  const genuinelySucceeded = rcpt.status === 1 || knownStatusZeroIsSuccess;
  const entry = {
    op: name,
    hash: tx.hash,
    gasUsed: rcpt.gasUsed.toString(),
    gasPriceWei: gasPrice.toString(),
    feeGasWei: (rcpt.gasUsed * gasPrice).toString(),
    status: genuinelySucceeded ? "success" : "reverted",
    receiptStatus: rcpt.status,
    contractAddress: rcpt.contractAddress,
  };
  if (knownStatusZeroIsSuccess && rcpt.status === 0) {
    entry.note =
      "receipt status=0 but genuinely succeeded (pallet-revive 1-byte return-data quirk), confirmed via read-only eth_call returning 0x01 before this tx";
  }
  results.push(entry);
  console.log(`  gas=${entry.gasUsed} status=${entry.status} (receipt status=${rcpt.status})`);
  return { rcpt, genuinelySucceeded };
}

let address = ADDRESS;
if (!address) {
  if (!BYTECODE) throw new Error("need BYTECODE (or ADDRESS to skip deploy)");
  const bytecode = readFileSync(BYTECODE);
  const { rcpt } = await record("deploy", () =>
    wallet.sendTransaction({ data: "0x" + bytecode.toString("hex"), gasLimit })
  );
  address = rcpt.contractAddress;
}
console.log(`contract: ${address}`);

const proof = "0x" + readFileSync(PROOF).toString("hex");
const pubBytes = readFileSync(PUBLIC_INPUTS);
const publicInputs = [];
for (let i = 0; i < pubBytes.length; i += 32) {
  publicInputs.push("0x" + pubBytes.subarray(i, i + 32).toString("hex"));
}
const iface = new ethers.Interface(["function verify(bytes,bytes32[]) returns (bool)"]);
const data = iface.encodeFunctionData("verify", [proof, publicInputs]);

// Read-only call first: some backends (pallet-revive) report a misleading
// status=0 on a mined tx that actually succeeded, so cross-check independently.
let readOnlySucceeded = false;
try {
  const callResult = await provider.call({ to: address, data });
  console.log(`read-only verify call result: ${callResult}`);
  readOnlySucceeded = callResult !== "0x" && !/^0x0+$/.test(callResult);
} catch (e) {
  console.log(`read-only verify call reverted: ${e.shortMessage || e.message}`);
}

try {
  await record("verify", () => wallet.sendTransaction({ to: address, data, gasLimit }), {
    knownStatusZeroIsSuccess: readOnlySucceeded,
  });
} catch (e) {
  console.log(`verify tx failed to broadcast: ${e.shortMessage || e.message}`);
  results.push({ op: "verify", contractAddress: address, status: "fails", error: e.shortMessage || e.message });
}

writeFileSync(
  OUT,
  JSON.stringify({ label: LABEL, rpc: RPC_URL, signer: wallet.address, results }, null, 2)
);
console.log(`written: ${OUT}`);
