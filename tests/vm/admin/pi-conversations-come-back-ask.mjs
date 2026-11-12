// What pi-conversations-come-back-vm-test.py asks a pi host through its
// control socket: one request, a line of JSON, and the line the host answers,
// printed as it came. A host that never answers fails the run rather than
// leaving the test to wait on it.
// Run as: node THIS SOCKET REQUEST
import { connect } from "node:net";

// Longer than a host takes to build a conversation and run its first turn.
const PATIENCE_MS = 150_000;
const NO_ANSWER = 2;
const [socket, request] = process.argv.slice(2);

setTimeout(() => {
  process.stderr.write(`${socket} gave no answer in ${PATIENCE_MS} ms\n`);
  process.exit(NO_ANSWER);
}, PATIENCE_MS).unref();

const answer = await new Promise((done, fail) => {
  const control = connect(socket, () => control.write(`${request}\n`));
  let heard = "";
  control.on("data", (data) => {
    heard += data;
    const end = heard.indexOf("\n");
    if (end >= 0) {
      control.end();
      done(heard.slice(0, end));
    }
  });
  control.on("error", fail);
});
process.stdout.write(`${answer}\n`);
