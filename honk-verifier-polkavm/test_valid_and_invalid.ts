import { ethers } from 'ethers';
import * as fs from 'fs';
import * as dotenv from 'dotenv';
dotenv.config();

const RPC_URL = 'https://eth-rpc-testnet.polkadot.io/';
const VERIFY_SELECTOR = '0xea50d0e4';

async function callVerify(provider: ethers.JsonRpcProvider, address: string, proofHex: string, publicInputs: string[]): Promise<number> {
  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  const encoded = abiCoder.encode(['bytes', 'bytes32[]'], [proofHex, publicInputs]);
  const result = await provider.call({ to: address, data: VERIFY_SELECTOR + encoded.slice(2) });
  return parseInt(result.slice(2, 4), 16); // first byte: 1=verified, 0=failed
}

async function main() {
  const { address } = JSON.parse(fs.readFileSync('deployment.json', 'utf8'));
  const proofBytes = fs.readFileSync('../circuit/target/proof');
  const proofHex = '0x' + proofBytes.toString('hex');
  const pubInputBytes = fs.readFileSync('../circuit/target/public_inputs');
  const publicInputs: string[] = [];
  for (let i = 0; i < pubInputBytes.length; i += 32) {
    publicInputs.push('0x' + pubInputBytes.slice(i, i + 32).toString('hex'));
  }

  const provider = new ethers.JsonRpcProvider(RPC_URL);
  console.log('Contract:', address, '\n');

  // Test 1: Valid proof -> should return 1
  const r1 = await callVerify(provider, address, proofHex, publicInputs);
  console.log(`Test 1 - Valid proof:              ${r1 === 1 ? 'PASS (verified)' : 'FAIL (got ' + r1 + ')'}`);

  // Test 2: Wrong public input -> should return 0
  const r2 = await callVerify(provider, address, proofHex, ['0x' + '00'.repeat(31) + 'ff']);
  console.log(`Test 2 - Wrong public input:       ${r2 === 0 ? 'PASS (rejected)' : 'FAIL (got ' + r2 + ')'}`);

  // Test 3: Corrupted sumcheck univariate -> should return 0
  const corrupt1 = Buffer.from(proofBytes);
  corrupt1[2000] ^= 0x01;
  const r3 = await callVerify(provider, address, '0x' + corrupt1.toString('hex'), publicInputs);
  console.log(`Test 3 - Corrupted univariate:     ${r3 === 0 ? 'PASS (rejected)' : 'FAIL (got ' + r3 + ')'}`);

  // Test 4: Corrupted sumcheck evaluation -> should return 0
  const corrupt2 = Buffer.from(proofBytes);
  corrupt2[4000] ^= 0x01;
  const r4 = await callVerify(provider, address, '0x' + corrupt2.toString('hex'), publicInputs);
  console.log(`Test 4 - Corrupted evaluation:     ${r4 === 0 ? 'PASS (rejected)' : 'FAIL (got ' + r4 + ')'}`);

  // Test 5: Corrupted commitment (EC point) -> should return 0
  const corrupt3 = Buffer.from(proofBytes);
  corrupt3[100] ^= 0x01;
  const r5 = await callVerify(provider, address, '0x' + corrupt3.toString('hex'), publicInputs);
  console.log(`Test 5 - Corrupted commitment:     ${r5 === 0 ? 'PASS (rejected)' : 'FAIL (got ' + r5 + ')'}`);
}

main().catch(console.error);
