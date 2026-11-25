// Shows that node, started with the word a realm's conversation is given
// for its heap growth, sets every heap limit V8 traces at the factor that
// word means, and that the same program without the word sets another,
// so the factor is the word's doing and not V8's own choice.
import { spawnSync } from "node:child_process";

const [, , node, word, factor] = process.argv;
if (node === undefined || word === undefined || factor === undefined) {
  throw new Error("usage: pi-conversations-take-heap-growth.mjs NODE WORD FACTOR");
}

// How long one node may take, so a hung one fails the build rather than
// holding it.
const ENDS_WITHIN_MS = 30_000;

// A conversation's heap limit, which decides the factor V8 picks itself.
const HEAP = "--max-old-space-size=4096";

// Keeps some memory live and churns more, then collects in full, so V8 sets
// a limit at least once from what is live.
const program = `
const kept = [];
for (let i = 0; i < 40; i++) kept.push(new Array(1e5).fill(i));
let churned = 0;
for (let i = 0; i < 400; i++) churned += new Array(1e5).fill(i).length;
gc();
process.stdout.write(String(kept.length + churned > 0));
`;

// The factor of each limit V8 traced, as it prints it.
function factorsOf(flags) {
  const run = spawnSync(node, [...flags, HEAP, "--trace-gc-verbose", "--expose-gc", "-e", program], {
    encoding: "utf8",
    timeout: ENDS_WITHIN_MS,
  });
  if (run.error !== undefined) {
    throw run.error;
  }
  if (run.status !== 0) {
    throw new Error(`node ${node} ${flags.join(" ")} exited ${run.status ?? run.signal}: ${run.stderr}`);
  }
  return [...run.stdout.matchAll(/Limit: old size: \d+ KB, new limit: \d+ KB \(([\d.]+)\)/g)].map(
    (found) => found[1],
  );
}

const given = factorsOf([word]);
if (given.length === 0 || given.some((one) => one !== factor)) {
  throw new Error(`node ${node} with ${word} traced limits at ${JSON.stringify(given)}, not all ${factor}`);
}
const own = factorsOf([]);
if (own.length === 0 || own.every((one) => one === factor)) {
  throw new Error(`node ${node} without ${word} traced limits at ${JSON.stringify(own)}, so ${factor} isn't the word's`);
}
