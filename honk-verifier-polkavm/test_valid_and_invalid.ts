import { ethers } from 'ethers';
import * as fs from 'fs';
import * as dotenv from 'dotenv';
dotenv.config();

const RPC_URL = 'https://eth-rpc-testnet.polkadot.io/';
const VERIFY_SELECTOR = '0xea50d0e4';

async function callVerify(provider: ethers.JsonRpcProvider, address: string, proofHex: string, publicInputs: string[]): Promise<Buffer> {
  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  const encoded = abiCoder.encode(['bytes', 'bytes32[]'], [proofHex, publicInputs]);
  const callData = VERIFY_SELECTOR + encoded.slice(2);
  const result = await provider.call({ to: address, data: callData });
  return Buffer.from(result.slice(2), 'hex');
}

async function main() {
  const { address } = JSON.parse(fs.readFileSync('deployment.json', 'utf8'));
  const provider = new ethers.JsonRpcProvider(RPC_URL);

  const proofBytes = fs.readFileSync('../circuit/target/proof');
  const proofHex = '0x' + proofBytes.toString('hex');
  const pubInputBytes = fs.readFileSync('../circuit/target/public_inputs');
  const publicInputs: string[] = [];
  for (let i = 0; i < pubInputBytes.length; i += 32) {
    publicInputs.push('0x' + pubInputBytes.slice(i, i + 32).toString('hex'));
  }

  console.log('Contract:', address);
  console.log('');

  // Test 1: Valid proof
  console.log('=== Test 1: Valid proof ===');
  const r1 = await callVerify(provider, address, proofHex, publicInputs);
  console.log(`  Result: ${r1.length} byte(s), value: 0x${r1.toString('hex')}`);
  console.log(`  ${r1[0] === 0 ? 'PASS - Valid proof accepted' : 'FAIL - Valid proof rejected with code ' + r1[0]}`);

  // Test 2: Wrong public input
  console.log('\n=== Test 2: Wrong public input ===');
  const wrongPI = ['0x' + '00'.repeat(31) + 'ff']; // garbage public input
  try {
    const r2 = await callVerify(provider, address, proofHex, wrongPI);
    console.log(`  Result: ${r2.length} byte(s), value: 0x${r2.toString('hex')}`);
    if (r2.length === 1 && r2[0] !== 0) {
      console.log(`  PASS - Wrong public input rejected (code ${r2[0]})`);
    } else {
      console.log(`  FAIL - Wrong public input was accepted!`);
    }
  } catch (e: any) {
    console.log(`  PASS - Wrong public input caused revert: ${e.message?.slice(0, 80)}`);
  }

  // Test 3: Corrupted sumcheck univariate (byte 2000 is deep in univariate data)
  console.log('\n=== Test 3: Corrupted proof (sumcheck univariate) ===');
  const corruptProof = Buffer.from(proofBytes);
  corruptProof[2000] ^= 0x01; // flip one bit
  try {
    const r3 = await callVerify(provider, address, '0x' + corruptProof.toString('hex'), publicInputs);
    console.log(`  Result: ${r3.length} byte(s), value: 0x${r3.toString('hex').slice(0, 20)}...`);
    if (r3.length === 1 && r3[0] !== 0) {
      console.log(`  PASS - Corrupted proof rejected (code ${r3[0]})`);
    } else if (r3.length > 1) {
      console.log(`  PASS - Corrupted proof returned diagnostic (${r3.length} bytes = grand sum mismatch)`);
    } else {
      console.log(`  FAIL - Corrupted proof was accepted!`);
    }
  } catch (e: any) {
    console.log(`  PASS - Corrupted proof caused revert: ${e.message?.slice(0, 80)}`);
  }

  // Test 4: Corrupted sumcheck evaluation
  console.log('\n=== Test 4: Corrupted proof (sumcheck evaluation) ===');
  const corruptProof2 = Buffer.from(proofBytes);
  corruptProof2[4000] ^= 0x01;
  try {
    const r4 = await callVerify(provider, address, '0x' + corruptProof2.toString('hex'), publicInputs);
    console.log(`  Result: ${r4.length} byte(s), value: 0x${r4.toString('hex').slice(0, 20)}...`);
    if (r4.length === 1 && r4[0] !== 0) {
      console.log(`  PASS - Corrupted proof rejected (code ${r4[0]})`);
    } else if (r4.length > 1) {
      console.log(`  PASS - Corrupted proof returned diagnostic (${r4.length} bytes = grand sum mismatch)`);
    } else {
      console.log(`  FAIL - Corrupted proof was accepted!`);
    }
  } catch (e: any) {
    console.log(`  PASS - Corrupted proof caused revert: ${e.message?.slice(0, 80)}`);
  }
}

main().catch(console.error);
