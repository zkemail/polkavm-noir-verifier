import { ethers } from 'ethers';
import * as fs from 'fs';
import * as dotenv from 'dotenv';
dotenv.config();

const RPC_URL = 'https://eth-rpc-testnet.polkadot.io/';
const VERIFY_SELECTOR = '0xea50d0e4';

// Sub-relation names for evals[0..25]
const SUBRELATION_NAMES = [
  'arith[0]',   // 0
  'arith[1]',   // 1
  'perm[0]',    // 2
  'perm[1]',    // 3
  'lookup[0]',  // 4
  'lookup[1]',  // 5
  'range[0]',   // 6
  'range[1]',   // 7
  'range[2]',   // 8
  'range[3]',   // 9
  'elliptic[0]',// 10
  'elliptic[1]',// 11
  'aux[0]',     // 12
  'aux[1]',     // 13
  'aux[2]',     // 14
  'aux[3]',     // 15
  'aux[4]',     // 16
  'aux[5]',     // 17
  'posext[0]',  // 18
  'posext[1]',  // 19
  'posext[2]',  // 20
  'posext[3]',  // 21
  'posint[0]',  // 22
  'posint[1]',  // 23
  'posint[2]',  // 24
  'posint[3]',  // 25
];

async function main() {
  const { address } = JSON.parse(fs.readFileSync('deployment.json', 'utf8'));
  console.log('Contract:', address);

  const proofBytes = fs.readFileSync('../circuit/target/proof');
  const pubInputBytes = fs.readFileSync('../circuit/target/public_inputs');
  const publicInputs: string[] = [];
  for (let i = 0; i < pubInputBytes.length; i += 32) {
    publicInputs.push('0x' + pubInputBytes.slice(i, i + 32).toString('hex'));
  }

  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  const encoded = abiCoder.encode(
    ['bytes', 'bytes32[]'],
    ['0x' + proofBytes.toString('hex'), publicInputs]
  );
  const callData = VERIFY_SELECTOR + encoded.slice(2);

  const provider = new ethers.JsonRpcProvider(RPC_URL);

  const result = await provider.call({ to: address, data: callData });
  console.log('Raw return length:', (result.length - 2) / 2, 'bytes');

  const data = Buffer.from(result.slice(2), 'hex');

  if (data.length === 864) {
    // Diagnostic mode: pow_partial_evaluation (32 bytes) + 26 × 32 bytes sub-relation evaluations
    const powVal = data.slice(0, 32).toString('hex');
    console.log(`\n  pow_partial_evaluation = 0x${powVal}`);
    console.log('\n--- Sub-relation evaluations (code 200 diagnostic) ---');
    for (let i = 0; i < 26; i++) {
      const val = data.slice(32 + i * 32, 32 + (i + 1) * 32).toString('hex');
      const name = SUBRELATION_NAMES[i] || `eval[${i}]`;
      console.log(`  evals[${i.toString().padStart(2)}] ${name.padEnd(12)} = 0x${val}`);
    }
    console.log('\nGrand sum mismatch — run compare_evals.ts for Solidity/Rust diff.');
  } else if (data.length === 832) {
    // Old format (pre-pow diagnostic) — 26 × 32 bytes sub-relation evaluations
    console.log('\n--- Sub-relation evaluations (old 832-byte format) ---');
    for (let i = 0; i < 26; i++) {
      const val = data.slice(i * 32, (i + 1) * 32).toString('hex');
      const name = SUBRELATION_NAMES[i] || `eval[${i}]`;
      console.log(`  evals[${i.toString().padStart(2)}] ${name.padEnd(12)} = 0x${val}`);
    }
  } else if (data.length === 1) {
    const byte = data[0];
    console.log('Return byte:', byte);
    if (byte === 0) console.log('SUMCHECK PASSED!');
    else if (byte >= 100 && byte <= 104) console.log(`SUMCHECK FAILED at round ${byte - 100}`);
    else if (byte === 200) console.log('Grand sum mismatch (old diagnostic)');
    else console.log('Unexpected value');
  } else if (data.length === 128) {
    // Old 128-byte diagnostic format
    console.log('Old 128-byte diagnostic:');
    console.log('  grand_sum    =', '0x' + data.slice(0, 32).toString('hex'));
    console.log('  round_target =', '0x' + data.slice(32, 64).toString('hex'));
    console.log('  eval[0]      =', '0x' + data.slice(64, 96).toString('hex'));
    console.log('  alphas[0]    =', '0x' + data.slice(96, 128).toString('hex'));
  } else {
    console.log('Unexpected return length:', data.length);
    console.log('Raw:', result);
  }
}
main().catch(console.error);
