// Shows that node, given --disable-sigusr1 as a pi host's unit gives it,
// outlives SIGUSR1 with its inspector shut. Any process of the account may
// send that signal, and an open inspector runs whatever code it is sent as
// the host, whose word mycelium takes for which conversation a spawner
// serves. A node without the flag is sent it too, as the control: its
// inspector opening shows the signal arrived and was acted on.
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

const [, , node] = process.argv;
if (node === undefined) {
  throw new Error("usage: pi-hosts-ignore-sigusr1.mjs NODE");
}

// How long the control's inspector may take to open, at most.
const OPENS_WITHIN_MS = 10_000;

// How long, at least, the shut node is watched once the control opened,
// and how many times the control's own opening time it is watched for.
const WATCHED_AT_LEAST_MS = 1000;
const WATCHED_TIMES_OVER = 3;

// How long it is between two questions to the shut node.
const ASKED_EVERY_MS = 50;

// How long the whole test may take, so a node that stops answering fails
// it rather than holding the build.
const ENDS_WITHIN_MS = 60_000;
setTimeout(() => {
  process.stderr.write(`a node stopped answering; gave up after ${ENDS_WITHIN_MS} ms\n`);
  process.exit(1);
}, ENDS_WITHIN_MS).unref();

// Each node says it's ready, then its inspector's address, or shut, for
// every line it's sent.
const program = `
const inspector = require("node:inspector");
require("node:readline")
  .createInterface({ input: process.stdin })
  .on("line", () => process.stdout.write((inspector.url() ?? "shut") + "\\n"));
process.stdout.write("ready\\n");
`;

/** A node run with `options`, and a way to ask it how its inspector stands. */
function started(options) {
  const run = spawn(node, [...options, "-e", program], {
    stdio: ["pipe", "pipe", "inherit"],
  });
  const lines = createInterface({ input: run.stdout })[Symbol.asyncIterator]();
  const said = async () => {
    const { value, done } = await lines.next();
    if (done) {
      throw new Error(`node ${options.join(" ")} ended, by ${run.signalCode ?? run.exitCode}`);
    }
    return value;
  };
  return {
    run,
    said,
    asked: () => {
      run.stdin.write("\n");
      return said();
    },
  };
}

const control = started(["--inspect-port=0"]);
// Both on a port of their own, so the shut node can't read shut only
// because another holds the default port.
const shut = started(["--disable-sigusr1", "--inspect-port=0"]);
try {
  for (const one of [control, shut]) {
    const first = await one.said();
    if (first !== "ready") {
      throw new Error(`a node first said ${first}`);
    }
  }
  shut.run.kill("SIGUSR1");
  control.run.kill("SIGUSR1");
  // Node opens its inspector on a thread of its own once the signal lands,
  // so the control is asked until it says so. The other was sent it first.
  const sent = Date.now();
  const until = sent + OPENS_WITHIN_MS;
  while (!(await control.asked()).startsWith("ws://")) {
    if (Date.now() > until) {
      throw new Error("the control's inspector never opened on SIGUSR1");
    }
  }
  // The shut node is watched for several times as long as the control
  // took, so a slow inspector, on a loaded builder, still shows.
  const opening = Date.now() - sent;
  const watchedUntil = Date.now() + Math.max(WATCHED_AT_LEAST_MS, opening * WATCHED_TIMES_OVER);
  do {
    const stands = await shut.asked();
    if (stands !== "shut") {
      throw new Error(`a node given --disable-sigusr1 opened its inspector: ${stands}`);
    }
    await new Promise((later) => setTimeout(later, ASKED_EVERY_MS));
  } while (Date.now() < watchedUntil);
} finally {
  control.run.kill();
  shut.run.kill();
}
