/**
 * Honk verifier generator.
 *
 * Reads a HonkVerifier.sol (from `bb write_solidity_verifier`), extracts
 * circuit-specific parameters, fills .rs.tmpl templates, copies static files,
 * and optionally builds the PolkaVM contract.
 */
import * as fs from 'fs';
import * as path from 'path';
import { copyDir, fillTemplate } from '../utils';
import { parseSolidity, VK_FIELD_ORDER, RUST_TO_SOL, ParsedHonkVerifier } from './parse_solidity';

// --- Heap size calculation ---

function calculateHeapKB(numPublicInputs: number): number {
  const CONST_PROOF_SIZE_LOG_N = 28;
  const NUMBER_OF_ENTITIES = 40;
  const OH = 32;

  const dataVec = 160 + 14080 + numPublicInputs * 32 + OH;
  const proofBytesCopy = 14080 + OH;
  const pubInputsVec = numPublicInputs * 32 + OH;
  const structs = (1900 + OH) + (14080 + OH) + (2900 + OH);
  const shplemini = (CONST_PROOF_SIZE_LOG_N * 32 + OH) * 2
    + (NUMBER_OF_ENTITIES + CONST_PROOF_SIZE_LOG_N + 2) * 32 + OH
    + (NUMBER_OF_ENTITIES + CONST_PROOF_SIZE_LOG_N + 2) * 64 + OH;

  const total = dataVec + proofBytesCopy + pubInputsVec + structs + shplemini;
  const withMargin = Math.ceil(total * 1.25);
  return Math.ceil(withMargin / 4096) * 4;
}

// --- Template value builders ---

function buildVkPoints(parsed: ParsedHonkVerifier): string {
  return VK_FIELD_ORDER.map(rustName => {
    const solName = RUST_TO_SOL[rustName];
    const pt = parsed.points.get(solName)!;
    return `    vk.${rustName} = g1(\n        "${pt.x}",\n        "${pt.y}",\n    );`;
  }).join('\n');
}

function buildPubInputParsing(numPub: number): string {
  if (numPub === 1) {
    return [
      `    let mut public_inputs: Vec<[u8; 32]> = alloc::vec![[0u8; 32]; 1];`,
      `    public_inputs[0].copy_from_slice(&data[arr_data_start..arr_data_start + 32]);`,
    ].join('\n');
  }
  const lines = [`    let mut public_inputs: Vec<[u8; 32]> = alloc::vec![[0u8; 32]; ${numPub}];`];
  for (let i = 0; i < numPub; i++) {
    lines.push(`    public_inputs[${i}].copy_from_slice(&data[arr_data_start + ${i * 32}..arr_data_start + ${(i + 1) * 32}]);`);
  }
  return lines.join('\n');
}

function buildSumcheckRounds(logN: number): string {
  const rounds: string[] = [];
  for (let i = 0; i < logN; i++) {
    rounds.push(`    // Round ${i}
    {
        let u = &proof.sumcheck_univariates[${i}];
        if !check_sum(u, round_target) { return ${100 + i}; }
        let ch = t.sumcheck_u_challenges[${i}];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[${i}], pow_partial_evaluation, ch);
    }`);
  }
  return rounds.join('\n');
}

// --- Main generate function ---

export function generate(solPath: string, outDir: string, build: boolean): void {
  console.log(`Reading ${solPath}...`);
  const solSrc = fs.readFileSync(solPath, 'utf8');
  const parsed = parseSolidity(solSrc);
  console.log(`  N=${parsed.N}, LOG_N=${parsed.LOG_N}, PUBLIC_INPUTS=${parsed.NUMBER_OF_PUBLIC_INPUTS}`);
  console.log(`  Found ${parsed.points.size} G1 points`);

  // Copy static files (everything in static/ → output root)
  const staticDir = path.join(__dirname, 'static');
  console.log(`Copying static files to ${outDir}...`);
  copyDir(staticDir, outDir);

  const srcDir = path.join(outDir, 'src');
  fs.mkdirSync(srcDir, { recursive: true });

  // Read and fill templates
  const templatesDir = path.join(__dirname, 'templates');
  const numPub = parsed.NUMBER_OF_PUBLIC_INPUTS;
  const heapKB = calculateHeapKB(numPub);

  console.log('Generating vk.rs...');
  const vkTmpl = fs.readFileSync(path.join(templatesDir, 'vk.rs.tmpl'), 'utf8');
  fs.writeFileSync(path.join(srcDir, 'vk.rs'), fillTemplate(vkTmpl, {
    CIRCUIT_SIZE: String(parsed.N),
    LOG_CIRCUIT_SIZE: String(parsed.LOG_N),
    PUBLIC_INPUTS_SIZE: String(numPub),
    VK_POINTS: buildVkPoints(parsed),
  }));

  console.log('Generating main.rs...');
  const mainTmpl = fs.readFileSync(path.join(templatesDir, 'main.rs.tmpl'), 'utf8');
  fs.writeFileSync(path.join(srcDir, 'main.rs'), fillTemplate(mainTmpl, {
    HEAP_KB: String(heapKB),
    NUM_PUB: String(numPub),
    LOG_N: String(parsed.LOG_N),
    PUB_INPUT_PARSING: buildPubInputParsing(numPub),
  }));

  console.log('Generating sumcheck.rs...');
  const sumcheckTmpl = fs.readFileSync(path.join(templatesDir, 'sumcheck.rs.tmpl'), 'utf8');
  fs.writeFileSync(path.join(srcDir, 'sumcheck.rs'), fillTemplate(sumcheckTmpl, {
    SUMCHECK_ROUNDS: buildSumcheckRounds(parsed.LOG_N),
  }));

  console.log(`\nDone! Output directory: ${outDir}`);

  if (build) {
    console.log('\nBuilding...');
    const { execSync } = require('child_process');
    try {
      execSync('cargo build --release', { cwd: outDir, stdio: 'inherit' });
      const binName = 'honk_verifier';
      execSync(
        `polkatool link --strip --min-stack-size 65536 --output ${binName}.polkavm target/riscv64emac-unknown-none-polkavm/release/${binName}.elf`,
        { cwd: outDir, stdio: 'inherit' },
      );
      console.log('Build complete!');
    } catch (e) {
      console.error('Build failed:', e);
      process.exit(1);
    }
  }
}
