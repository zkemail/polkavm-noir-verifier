import { ethers } from 'ethers';
import * as fs from 'fs';
import * as dotenv from 'dotenv';
dotenv.config();

const RPC_URL = 'https://eth-rpc-testnet.polkadot.io/';
const VERIFY_SELECTOR = '0xea50d0e4';

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
  console.log('Raw return:', result);
  const byte = parseInt(result.slice(2, 4) || 'ff', 16);
  console.log('Return byte:', byte);
  if (byte === 0) console.log('SUMCHECK PASSED!');
  else if (byte >= 100 && byte <= 104) console.log(`SUMCHECK FAILED at round ${byte - 100}`);
  else if (byte === 200) console.log('All rounds passed but grand sum mismatch');
  else console.log('Unexpected value');
}
main().catch(console.error);
