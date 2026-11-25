// Shows that node, started with the word a realm's conversation is given
// for its heap growth, grows every heap limit V8 traces by the factor that
// word means, and that the same program without the word grows by another,
// so the factor is the word's doing and not V8's own choice. It reads the
// factor, not the limit, which V8 also floors and caps.
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

// The factor V8 traced before setting a limit.
const FACTOR = /\[HeapController\] factor ([\d.]+) based on/u;
// A heap limit V8 traced, with the factor node 22's V8 prints on its line.
const LIMIT = /\[HeapController\] Limit: old size: \d+ KB, new limit: \d+ KB(?: \(([\d.]+)\))?$/u;

// The factor each heap limit in a trace was grown by. Node 22's V8 prints
// it on the limit's line, after its own choice on the line before; node
// 24's prints it only on the line before, already the word's. A factor
// serves one limit, and a limit with none of its own is refused, so no
// limit is read with another's.
function factorsIn(trace) {
  const factors = [];
  let chosen;
  for (const line of trace.split("\n")) {
    const choice = FACTOR.exec(line);
    if (choice !== null) {
      chosen = choice[1];
    }
    const limit = LIMIT.exec(line);
    if (limit !== null) {
      const applied = limit[1] ?? chosen;
      if (applied === undefined) {
        throw new Error(`a heap limit was traced with no factor of its own: ${line}`);
      }
      factors.push(applied);
      chosen = undefined;
    }
  }
  return factors;
}

// Two limits after one factor: reading the second with the first's factor
// would pass a limit the word never reached.
const twoLimitsOneFactor = [
  "[HeapController] factor 1.3 based on mu=0.970",
  "[HeapController] Limit: old size: 1 KB, new limit: 2 KB",
  "[HeapController] Limit: old size: 1 KB, new limit: 2 KB",
].join("\n");
let refused = false;
try {
  factorsIn(twoLimitsOneFactor);
} catch {
  refused = true;
}
if (!refused) {
  throw new Error("the trace's reading gave a limit another limit's factor");
}

// The factor each heap limit was grown by in node run with `flags`.
function factorsOf(flags) {
  const run = spawnSync(
    node,
    [...flags, HEAP, "--trace-gc-verbose", "--expose-gc", "-e", program],
    {
      encoding: "utf8",
      timeout: ENDS_WITHIN_MS,
    },
  );
  if (run.error !== undefined) {
    throw run.error;
  }
  if (run.status !== 0) {
    throw new Error(
      `node ${node} ${flags.join(" ")} exited ${run.status ?? run.signal}: ${run.stderr}`,
    );
  }
  return factorsIn(run.stdout);
}

const given = factorsOf([word]);
if (given.length === 0 || given.some((one) => one !== factor)) {
  throw new Error(
    `node ${node} with ${word} traced limits at ${JSON.stringify(given)}, not all ${factor}`,
  );
}
const own = factorsOf([]);
if (own.length === 0 || own.every((one) => one === factor)) {
  throw new Error(
    `node ${node} without ${word} traced limits at ${JSON.stringify(own)}, so ${factor} isn't the word's`,
  );
}
