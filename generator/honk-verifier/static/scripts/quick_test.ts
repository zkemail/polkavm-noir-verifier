import { ethers } from 'ethers';
import * as fs from 'fs';
import * as path from 'path';

import * as dotenv from 'dotenv';


const __dirname = path.dirname(process.argv[1]);
const ROOT = path.join(__dirname, '..');
dotenv.config({ path: path.join(ROOT, '.env') });

const RPC_URL = 'https://eth-rpc-testnet.polkadot.io/';
const VERIFY_SELECTOR = '0xea50d0e4';

async function main() {
  const { address } = JSON.parse(fs.readFileSync(path.join(ROOT, 'deployment.json'), 'utf8'));
  console.log('Contract:', address);

  const proofBytes = fs.readFileSync(path.join(ROOT, 'proof'));
  const pubInputBytes = fs.readFileSync(path.join(ROOT, 'public_inputs'));
  const publicInputs: string[] = [];
  for (let i = 0; i < pubInputBytes.length; i += 32) {
    publicInputs.push('0x' + pubInputBytes.slice(i, i + 32).toString('hex'));
  }

  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  const encoded = abiCoder.encode(['bytes', 'bytes32[]'], ['0x' + proofBytes.toString('hex'), publicInputs]);
  const callData = VERIFY_SELECTOR + encoded.slice(2);

  const provider = new ethers.JsonRpcProvider(RPC_URL);

  console.log('Calling verify()...');
  try {
    const result = await provider.call({ to: address, data: callData });
    const data = Buffer.from(result.slice(2), 'hex');
    console.log(`Return: ${data.length} byte(s), value: 0x${data.toString('hex')}`);

    if (data.length === 1) {
      if (data[0] === 1) {
        console.log('VERIFIED! Proof is valid.');
      } else if (data[0] === 0) {
        console.log('FAILED: Proof verification failed.');
      } else {
        console.log(`Unknown return code: ${data[0]}`);
      }
    } else {
      console.log('Unexpected return length');
    }
  } catch (e: any) {
    console.log('Call failed:', e.message?.slice(0, 100));
  }
}

main().catch(console.error);
