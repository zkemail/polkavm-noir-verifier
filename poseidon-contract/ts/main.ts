import { ABI } from "./erc20";
import { readFileSync } from "fs";
import { ethers, JsonRpcProvider, Wallet, Contract } from "ethers";
import { config } from "dotenv";

function generateRandomEthersWallet(provider: JsonRpcProvider) {
  const account = ethers.Wallet.createRandom();
  const wallet = new ethers.Wallet(account.privateKey, provider);
  return wallet;
}

function uint8ArrayToHex(arr: Uint8Array): string {
  return Array.from(arr)
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function getBytecode(): string {
  const path = "../pos.polkavm";
  const bytecode = readFileSync(path);
  const bytecodeHex = uint8ArrayToHex(bytecode);
  const validBytecode = bytecodeHex.replace("0x", "");
  return validBytecode;
}

async function deploy(provider: JsonRpcProvider, etherWallet: Wallet) {
  const bytecode = getBytecode();
  const factory = new ethers.ContractFactory(ABI, bytecode, etherWallet);
  const contract = await factory.deploy();

  await contract.waitForDeployment();
  const contractAddress = contract.target.toString();
  console.log("contract address is: ", contractAddress);
  return contractAddress;
}

async function main() {
  const url = "https://eth-rpc-testnet.polkadot.io/";
  const provider = new ethers.JsonRpcProvider(url);
  console.log(`connected to rpc`);
  config();
  let privateKey = process.env.AH_PRIV_KEY || "";
  const walletClient = new Wallet(privateKey, provider);
  console.log(`connected to wallet`);
  const contractAddress = await deploy(provider, walletClient);
  console.log(`contract address is: `, contractAddress);
  const contract = new Contract(contractAddress, ABI, walletClient);
  console.log(`contract object created, you can now use your contract!`);
  // wallet from env private key
  const walletAddress = await walletClient.getAddress();
  console.log(`all done!`);
}

main();
