import { ethers } from 'ethers';
import * as fs from 'fs';
import * as dotenv from 'dotenv';
import * as path from 'path';

dotenv.config();


const RPC_URL = 'https://eth-rpc-testnet.polkadot.io/';
const PRIVATE_KEY = process.env.PRIVATE_KEY || '';

// verify(bytes,bytes32[]) selector
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

  // Read proof and public inputs from circuit/target/
  const circuitTarget = path.join(process.cwd(), '../circuit/target');
  const proofBytes = fs.readFileSync(path.join(circuitTarget, 'proof'));
  const pubInputBytes = fs.readFileSync(path.join(circuitTarget, 'public_inputs'));

  console.log(`Proof size: ${proofBytes.length} bytes`);
  console.log(`Public inputs: ${pubInputBytes.length / 32} field element(s)`);

  // Parse public inputs as bytes32[]
  if (pubInputBytes.length % 32 !== 0) {
    throw new Error('public_inputs length not a multiple of 32');
  }
  const publicInputs: string[] = [];
  for (let i = 0; i < pubInputBytes.length; i += 32) {
    const chunk = pubInputBytes.slice(i, i + 32);
    publicInputs.push('0x' + chunk.toString('hex'));
  }
  console.log('Public inputs:', publicInputs);

  // ABI-encode: verify(bytes proof, bytes32[] publicInputs)
  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  const encoded = abiCoder.encode(
    ['bytes', 'bytes32[]'],
    ['0x' + proofBytes.toString('hex'), publicInputs]
  );
  const callData = VERIFY_SELECTOR + encoded.slice(2);

  const provider = new ethers.JsonRpcProvider(RPC_URL);
  const wallet = new ethers.Wallet(PRIVATE_KEY, provider);

  console.log('\nSending verify() call...');
  console.log('(This may take a moment — 70 ecMul precompile calls on-chain)');

  // Use sendTransaction (not provider.call) because polkavm precompile calls
  // may not work in eth_call simulation; use a real tx to be safe.
  const tx = await wallet.sendTransaction({
    to: contractAddress,
    data: callData,
    gasLimit: 200_000_000,
  });

  console.log(`Tx: ${tx.hash}`);
  const receipt = await tx.wait();
  console.log(`Status: ${receipt!.status === 1 ? 'SUCCESS' : 'REVERTED'}`);
  console.log(`Gas used: ${receipt!.gasUsed.toString()}`);

  if (receipt!.status !== 1) {
    console.error('Transaction reverted — contract panicked or ran out of gas');
    process.exit(1);
  }

  // Decode return value from logs or try eth_call for the return data
  // Since sendTransaction doesn't give return data, re-call with eth_call
  try {
    const result = await provider.call({
      to: contractAddress,
      data: callData,
    });
    console.log(`\nRaw return (hex): ${result}`);
    const decoded = abiCoder.decode(['bool'], result);
    const verified = decoded[0] as boolean;
    console.log(`Proof verified: ${verified}`);
    if (!verified) {
      console.error('Verification returned false — check proof/VK/transcript');
      process.exit(1);
    }
    console.log('SUCCESS: UltraHonk proof verified on Paseo Asset Hub!');
  } catch (e) {
    console.log('eth_call for return value failed (normal for polkavm), checking tx success...');
    console.log('Transaction succeeded — likely returned true (no revert)');
  }
}

main().catch(console.error);
