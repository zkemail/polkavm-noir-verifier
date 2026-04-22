/** Parse a HonkVerifier.sol file to extract circuit-specific parameters. */

export interface G1Point { x: string; y: string }

export interface ParsedHonkVerifier {
  N: number;
  LOG_N: number;
  NUMBER_OF_PUBLIC_INPUTS: number;
  points: Map<string, G1Point>;
}

/** Solidity camelCase → Rust snake_case field name mapping. */
export const SOL_TO_RUST: Record<string, string> = {
  ql: 'ql', qr: 'qr', qo: 'qo', q4: 'q4', qm: 'qm', qc: 'qc',
  qArith: 'q_arith', qDeltaRange: 'q_delta_range', qElliptic: 'q_elliptic',
  qAux: 'q_aux', qLookup: 'q_lookup',
  qPoseidon2External: 'q_poseidon2_external', qPoseidon2Internal: 'q_poseidon2_internal',
  s1: 's1', s2: 's2', s3: 's3', s4: 's4',
  t1: 't1', t2: 't2', t3: 't3', t4: 't4',
  id1: 'id1', id2: 'id2', id3: 'id3', id4: 'id4',
  lagrangeFirst: 'lagrange_first', lagrangeLast: 'lagrange_last',
};

/** Rust field names in VK struct assignment order. */
export const VK_FIELD_ORDER = [
  'ql', 'qr', 'qo', 'q4', 'qm', 'qc', 'q_arith', 'q_delta_range', 'q_elliptic',
  'q_aux', 'q_lookup', 'q_poseidon2_external', 'q_poseidon2_internal',
  's1', 's2', 's3', 's4', 't1', 't2', 't3', 't4',
  'id1', 'id2', 'id3', 'id4', 'lagrange_first', 'lagrange_last',
];

/** Reverse map: Rust name → Solidity name. */
export const RUST_TO_SOL: Record<string, string> = {};
for (const [sol, rust] of Object.entries(SOL_TO_RUST)) {
  RUST_TO_SOL[rust] = sol;
}

const EXPECTED_POINTS = [
  'ql', 'qr', 'qo', 'q4', 'qm', 'qc', 'qArith', 'qDeltaRange', 'qElliptic',
  'qAux', 'qLookup', 'qPoseidon2External', 'qPoseidon2Internal',
  's1', 's2', 's3', 's4', 't1', 't2', 't3', 't4',
  'id1', 'id2', 'id3', 'id4', 'lagrangeFirst', 'lagrangeLast',
];

export function parseSolidity(src: string): ParsedHonkVerifier {
  const constMatch = (name: string): number => {
    const m = src.match(new RegExp(`uint256\\s+constant\\s+${name}\\s*=\\s*([\\d_]+)`));
    if (!m) throw new Error(`Cannot find constant ${name} in HonkVerifier.sol`);
    return parseInt(m[1].replace(/_/g, ''), 10);
  };

  const N = constMatch('N');
  const LOG_N = constMatch('LOG_N');
  const NUMBER_OF_PUBLIC_INPUTS = constMatch('NUMBER_OF_PUBLIC_INPUTS');

  const pointRegex = /(\w+):\s*Honk\.G1Point\(\{\s*x:\s*uint256\(0x([0-9a-fA-F]+)\),\s*y:\s*uint256\(0x([0-9a-fA-F]+)\)\s*\}\)/g;
  const points = new Map<string, G1Point>();
  let m;
  while ((m = pointRegex.exec(src)) !== null) {
    points.set(m[1], { x: m[2].padStart(64, '0'), y: m[3].padStart(64, '0') });
  }

  for (const p of EXPECTED_POINTS) {
    if (!points.has(p)) throw new Error(`Missing G1 point: ${p}`);
  }

  return { N, LOG_N, NUMBER_OF_PUBLIC_INPUTS, points };
}
