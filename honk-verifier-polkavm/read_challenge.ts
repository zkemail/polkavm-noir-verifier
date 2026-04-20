import { ethers } from 'ethers';
import * as fs from 'fs';
import * as dotenv from 'dotenv';
import * as path from 'path';
dotenv.config();

const RPC_URL = 'https://eth-rpc-testnet.polkadot.io/';
const PRIVATE_KEY = process.env.PRIVATE_KEY || '';
const VERIFY_SELECTOR = '0xea50d0e4';

async function main() {
    const { address } = JSON.parse(fs.readFileSync('deployment.json', 'utf8'));
    const proofBytes = fs.readFileSync(path.join('..', 'circuit', 'target', 'proof'));
    const pubInputBytes = fs.readFileSync(path.join('..', 'circuit', 'target', 'public_inputs'));
    
    const publicInputs: string[] = [];
    for (let i = 0; i < pubInputBytes.length; i += 32) {
        publicInputs.push('0x' + pubInputBytes.slice(i, i+32).toString('hex'));
    }
    
    const abiCoder = ethers.AbiCoder.defaultAbiCoder();
    const encoded = abiCoder.encode(['bytes', 'bytes32[]'], ['0x' + proofBytes.toString('hex'), publicInputs]);
    const callData = VERIFY_SELECTOR + encoded.slice(2);
    
    const provider = new ethers.JsonRpcProvider(RPC_URL);
    const wallet = new ethers.Wallet(PRIVATE_KEY, provider);
    
    const tx = await wallet.sendTransaction({ to: address, data: callData, gasLimit: 200_000_000 });
    console.log('Tx:', tx.hash);
    await tx.wait();
    
    // Get return data
    const result = await provider.call({ to: address, data: callData });
    console.log('Raw return (challenge bytes):', result);
    console.log('Expected (from TypeScript):    0x00000000000000000000000000000000a310e9b340ef635f1f087098ab161d74');
}
main().catch(console.error);
