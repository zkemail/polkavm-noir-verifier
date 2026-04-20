import { ethers } from 'ethers';
import * as fs from 'fs';
import * as dotenv from 'dotenv';
import * as path from 'path';

dotenv.config();

const RPC_URL = 'https://eth-rpc-testnet.polkadot.io/';
const PRIVATE_KEY = process.env.PRIVATE_KEY || '';
const VERIFY_SELECTOR = '0xea50d0e4';

async function main() {
  if (!PRIVATE_KEY) {
    console.error('PRIVATE_KEY not found in .env');
    process.exit(1);
  }

  const deploymentPath = path.join(process.cwd(), 'deployment.json');
  if (!fs.existsSync(deploymentPath)) {
    console.error('deployment.json not found — run deploy.ts first');
    process.exit(1);
  }
  const { address: contractAddress } = JSON.parse(fs.readFileSync(deploymentPath, 'utf8'));
  console.log('Contract:', contractAddress);

  const circuitTarget = path.join(process.cwd(), '../circuit/target');
  const proofBytes = Buffer.from(fs.readFileSync(path.join(circuitTarget, 'proof')));
  const pubInputBytes = fs.readFileSync(path.join(circuitTarget, 'public_inputs'));

  if (proofBytes.length < 128) {
    throw new Error(`Proof too short: ${proofBytes.length}`);
  }
  // Corrupt one byte in-memory to force verification failure.
  const flipIndex = 64;
  proofBytes[flipIndex] ^= 0x01;
  console.log(`Mutated proof byte at index ${flipIndex}`);

  if (pubInputBytes.length % 32 !== 0) {
    throw new Error('public_inputs length not a multiple of 32');
  }
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
  const wallet = new ethers.Wallet(PRIVATE_KEY, provider);

  console.log('Sending verify() with corrupted proof...');
  const tx = await wallet.sendTransaction({
    to: contractAddress,
    data: callData,
    gasLimit: 200_000_000,
  });
  console.log(`Tx: ${tx.hash}`);
  const receipt = await tx.wait();
  const reverted = receipt?.status !== 1;
  console.log(`Status: ${reverted ? 'REVERTED' : 'SUCCESS'}`);
  console.log(`Gas used: ${receipt?.gasUsed.toString()}`);

  if (!reverted) {
    console.error('FAIL: Corrupted proof unexpectedly succeeded.');
    process.exit(1);
  }

  console.log('PASS: Corrupted proof was rejected (reverted) as expected.');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
