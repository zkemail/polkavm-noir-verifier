# Benchmarks

Raw data and scripts behind the comparison table in [`04_gas_optimization_benchmark_report.md`](../04_gas_optimization_benchmark_report.md).

- `compile.sh` - compiles all four legs' bytecode from the committed `fixtures/*/target/HonkVerifier.sol` sources into `build/`. Run this first.
- `bench.js` - deploys a compiled bytecode file (or reuses an existing address) and submits a real `verify(bytes,bytes32[])` transaction, writing the result to a JSON file. Run with Node 18+ (uses `ethers`).
- `pvm-native.json`, `pvm-resolc.json`, `revm.json`, `evm.json` - real transaction results (hash, gas used, gas price, fee) for each leg, fetched directly from each chain's RPC after running `bench.js`.
- `aggregate.js` - reads the four JSON files and prints the comparison table. Run with `node aggregate.js`; the output matches the table in the report exactly, so the report's numbers are reproducible from this committed data rather than hand-copied.

## Setup

```bash
npm install          # installs ethers + @parity/resolc
./compile.sh          # produces build/{pvm-native,pvm-resolc,revm-evm}/{noir-circuit,zkemail}/...
```

`compile.sh` additionally needs `cargo`, `polkatool`, and `solc` (>=0.8.21; this project used 0.8.36) on `PATH` - the same toolchain the rest of this repo already requires, not installed by `npm install`.

## Reproducing a single leg

```bash
RPC_URL=<chain rpc> \
PK=0x<burner private key> \
EXPECTED=0x<burner address, safety guard> \
BYTECODE=<path to raw contract bytecode, e.g. build/pvm-native/noir-circuit/honk_verifier.polkavm> \
PROOF=<path to fixture proof> \
PUBLIC_INPUTS=<path to fixture public_inputs> \
OUT=out.json \
LABEL="my run" \
node bench.js
```

Omit `BYTECODE` and pass `ADDRESS=0x<existing contract>` to re-run just the verify step against an already-deployed contract.

`GAS_LIMIT` defaults to 60,000,000, required on Paseo where gas estimation rejects contract-creation transactions outright without an explicit limit; Sepolia does not need it.

## A note on deploy gas: cold vs. warm

Paseo (pallet-revive) stores contract code once per unique bytecode hash; a later deployment of *identical* bytecode reuses the stored code instead of paying to store it again, and is substantially cheaper as a result. This isn't specific to PVM bytecode - it showed up identically on the REVM leg. Real Ethereum has no equivalent: `CODEDEPOSIT` charges the same per-byte cost every time, regardless of whether identical bytecode exists elsewhere.

The committed JSON reports **warm** (steady-state) deploy gas throughout, since that's what a real re-run of this benchmark will always observe from now on - every leg's bytecode is already "known" on Paseo. We don't have a genuine first-ever ("cold") price for every leg on equal footing (native PVM's bytecode was already on-chain, from the earlier M1.5 evidence deployment, before this benchmark session began, so no cold price for it was ever observable), so this doc reports warm pricing uniformly rather than mixing in cold numbers for only some legs.
