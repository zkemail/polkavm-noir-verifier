import { ethers } from 'ethers';
import * as fs from 'fs';
import * as dotenv from 'dotenv';

dotenv.config();

const RPC_URL = 'https://eth-rpc-testnet.polkadot.io/';
const PRIVATE_KEY = process.env.PRIVATE_KEY || '';

async function main() {
  if (!PRIVATE_KEY) {
    console.error('PRIVATE_KEY not found in .env');
    process.exit(1);
  }

  const provider = new ethers.JsonRpcProvider(RPC_URL);
  const wallet = new ethers.Wallet(PRIVATE_KEY, provider);

  console.log('Deployer:', wallet.address);
  const balance = await provider.getBalance(wallet.address);
  console.log('Balance:', ethers.formatEther(balance), 'PAS\n');

  const bytecode = fs.readFileSync('honk_verifier.polkavm');
  console.log(`Contract size: ${bytecode.length} bytes`);

  const tx = await wallet.sendTransaction({
    data: '0x' + bytecode.toString('hex'),
    gasLimit: 60_000_000,
  });

  console.log(`Tx: ${tx.hash}`);
  const receipt = await tx.wait();
  const contractAddress = receipt!.contractAddress!;
  console.log(`Contract deployed: ${contractAddress}`);

  fs.writeFileSync(
    'deployment.json',
    JSON.stringify({ address: contractAddress, timestamp: new Date().toISOString() }, null, 2)
  );
  console.log('Saved to deployment.json');
}

main().catch(console.error);
