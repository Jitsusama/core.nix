// Shows that node 22, started as a pi host whose realm sets
// asyncContextFrame is, keeps each conversation's AsyncLocalStorage value
// across awaits, timers and promises settled elsewhere, and that a node
// which refuses the flag, as 24 does, ends at once saying so. A host on
// such a node would end as it starts, which is why the option names its
// version floor.
import { spawnSync } from "node:child_process";

const [, , node, expected] = process.argv;
if (node === undefined || (expected !== "keeps" && expected !== "refuses")) {
  throw new Error("usage: pi-hosts-keep-async-context.mjs NODE keeps|refuses");
}

// How long one node may take, so a hung one fails the build rather than
// holding it.
const ENDS_WITHIN_MS = 30_000;

// How many conversations run at once, each awaiting in its own way.
const CONVERSATIONS = 50;

// Each conversation's value must outlive an await, a timer, a promise
// another conversation settles, and a microtask queued from inside it.
const program = `
const { AsyncLocalStorage } = require("node:async_hooks");
const store = new AsyncLocalStorage();
const settlers = [];
const later = (ms) => new Promise((done) => setTimeout(done, ms));
async function conversation(id) {
  await null;
  await later(id % 7);
  await new Promise((done) => settlers.push(done));
  await new Promise((done) => queueMicrotask(done));
  return store.getStore() === id ? null : id + " read " + store.getStore();
}
const runs = [];
for (let id = 0; id < ${CONVERSATIONS}; id++) runs.push(store.run(id, () => conversation(id)));
setTimeout(() => settlers.splice(0).reverse().forEach((settle) => settle()), 20);
Promise.all(runs).then((lost) => {
  const wrong = lost.filter((one) => one !== null);
  if (store.getStore() !== undefined) wrong.push("outside read " + store.getStore());
  process.stdout.write(wrong.length === 0 ? "kept" : "lost: " + wrong.join(", "));
});
`;

const run = spawnSync(
  node,
  ["--disable-sigusr1", "--experimental-async-context-frame", "-e", program],
  { encoding: "utf8", timeout: ENDS_WITHIN_MS },
);
if (run.error !== undefined) {
  throw run.error;
}

if (expected === "keeps") {
  if (run.status !== 0 || run.stdout !== "kept") {
    throw new Error(
      `node ${node} with V8's frame didn't keep every value: exited ${run.status ?? run.signal}, said ${JSON.stringify(run.stdout)}, ${run.stderr}`,
    );
  }
} else if (run.status === 0 || !run.stderr.includes("--experimental-async-context-frame")) {
  throw new Error(
    `node ${node} was expected to refuse the frame's flag: exited ${run.status ?? run.signal}, said ${JSON.stringify(run.stderr)}`,
  );
}
