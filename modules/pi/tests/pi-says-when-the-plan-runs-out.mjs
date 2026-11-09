// What pi says when a Claude plan limit is used up, asked of the patched
// claude-code provider whose folder is the first argument: which limit, when
// it resets, that nothing went to extra usage, and "billing", which pi's
// retry classifier never retries. Anything else the provider passes on.
import assert from "node:assert/strict";

const [, , provider] = process.argv;
const { describePlanLimit } = await import(`${provider}/src/billing.ts`);

const said = (message) => describePlanLimit(message);

const usage = said("Claude Code API error: You've hit your usage limit · resets 3pm");
assert.match(
  usage,
  /^Claude plan limit reached: your usage limit is used up until it resets 3pm\./u,
);
assert.match(usage, /nothing is charged to extra usage/u);
assert.match(usage, /billing/u);

assert.match(
  said("Claude Code API error: You've hit your weekly limit · resets Oct 30, 9am"),
  /your weekly limit is used up until it resets Oct 30, 9am\./u,
);
assert.match(said("You've reached your Opus limit"), /your Opus limit is used up\. Pi stopped/u);

assert.equal(said("Claude Code request failed: error_during_execution"), undefined);
assert.equal(said("rate limit exceeded"), undefined);
