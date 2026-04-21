import { ethers } from 'ethers';
import * as fs from 'fs';
import * as dotenv from 'dotenv';
dotenv.config({ path: '../honk-verifier-polkavm/.env' });

const RPC = 'https://eth-rpc-testnet.polkadot.io/';

async function main() {
  const provider = new ethers.JsonRpcProvider(RPC);
  const wallet = new ethers.Wallet(process.env.PRIVATE_KEY!, provider);

  // Deploy
  const bytecode = fs.readFileSync('precompile_test.polkavm');
  console.log(`Deploying ${bytecode.length} bytes...`);
  const tx = await wallet.sendTransaction({ data: '0x' + bytecode.toString('hex'), gasLimit: 60_000_000 });
  const receipt = await tx.wait();
  const addr = receipt!.contractAddress!;
  console.log(`Deployed: ${addr}\n`);

  // Call each function as a read call (eth_call)
  for (const [name, sel] of [['ecAdd', '00000001'], ['ecMul', '00000002'], ['ecPairing', '00000003']]) {
    console.log(`--- ${name} ---`);
    try {
      const result = await provider.call({ to: addr, data: '0x' + sel });
      const buf = Buffer.from(result.slice(2), 'hex');
      console.log(`  status: ${buf[0]}  (0=OK, 5=OutOfResources)`);
      console.log(`  output: 0x${buf.slice(1).toString('hex')}`);
    } catch (e: any) {
      console.log(`  error: ${e.message?.slice(0, 80)}`);
    }
  }
}
main().catch(console.error);
