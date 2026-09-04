// Combine pvm-native.json + pvm-resolc.json + revm.json + evm.json into the
// comparison table published in 04_gas_optimization_benchmark_report.md.
// Run from this directory: node aggregate.js
import { readFileSync } from "fs";

const LEGS = ["pvm-native", "pvm-resolc", "revm", "evm"];
const data = Object.fromEntries(LEGS.map((f) => [f, JSON.parse(readFileSync(`${f}.json`))]));

function cell(legFile, fixture, op) {
  const r = data[legFile].results.find((r) => r.fixture === fixture && r.op === op);
  if (!r) return "-";
  if (r.status !== "success") return `fails - ${r.error || r.status}`;
  return Number(r.gasUsed).toLocaleString();
}

const headers = LEGS.map((f) => data[f].leg);
console.log(`| | ${headers.join(" | ")} |`);
console.log(`| --- | ${headers.map(() => "---:").join(" | ")} |`);
for (const op of ["deploy", "verify"]) {
  for (const fixture of ["noir-circuit", "zkemail"]) {
    const label = `${op === "deploy" ? "Deploy" : "Verify"} gas - ${fixture}`;
    console.log(`| ${label} | ${LEGS.map((f) => cell(f, fixture, op)).join(" | ")} |`);
  }
}
