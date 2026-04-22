/**
 * PolkaVM verifier generator — main entry point.
 *
 * Routes to the appropriate template-specific generator based on the subcommand.
 *
 * Usage:
 *   npx ts-node generate_verifier.ts honk --sol <HonkVerifier.sol> --out <dir> [--build]
 *   npx ts-node generate_verifier.ts groth16 ...  (future)
 */
import * as path from 'path';

function printUsage(): never {
  console.error(`Usage: ts-node generate_verifier.ts <type> [options]

Types:
  honk      Generate UltraHonk verifier from HonkVerifier.sol

Options:
  --sol <path>   Path to HonkVerifier.sol (from \`bb write_solidity_verifier\`)
  --out <path>   Output directory for the generated project
  --build        Build the contract after generating

Examples:
  npx ts-node generate_verifier.ts honk --sol ../fixtures/noir-circuit/target/HonkVerifier.sol --out ../contracts/honk-verifier --build`);
  process.exit(1);
}

function parseArgs(): { type: string; sol: string; out: string; build: boolean } {
  const args = process.argv.slice(2);
  if (args.length === 0) printUsage();

  const type = args[0];
  let sol = '', out = '', build = false;
  for (let i = 1; i < args.length; i++) {
    if (args[i] === '--sol' && args[i + 1]) sol = args[++i];
    else if (args[i] === '--out' && args[i + 1]) out = args[++i];
    else if (args[i] === '--build') build = true;
  }
  if (!sol || !out) printUsage();
  return { type, sol, out, build };
}

function main() {
  const { type, sol, out, build } = parseArgs();
  const solPath = path.resolve(sol);
  const outDir = path.resolve(out);

  switch (type) {
    case 'honk': {
      const { generate } = require('./honk-verifier/generate');
      generate(solPath, outDir, build);
      break;
    }
    // Future: case 'groth16': { ... }
    default:
      console.error(`Unknown verifier type: ${type}`);
      console.error('Available types: honk');
      process.exit(1);
  }
}

main();
