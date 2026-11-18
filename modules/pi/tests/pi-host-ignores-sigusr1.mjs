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
  throw new Error("usage: pi-host-ignores-sigusr1.mjs NODE");
}

// How long the control's inspector may take to open, at most.
const OPENS_WITHIN_MS = 10_000;

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
const shut = started(["--disable-sigusr1"]);
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
  const until = Date.now() + OPENS_WITHIN_MS;
  while (!(await control.asked()).startsWith("ws://")) {
    if (Date.now() > until) {
      throw new Error("the control's inspector never opened on SIGUSR1");
    }
  }
  const stands = await shut.asked();
  if (stands !== "shut") {
    throw new Error(`a node given --disable-sigusr1 opened its inspector: ${stands}`);
  }
} finally {
  control.run.kill();
  shut.run.kill();
}
