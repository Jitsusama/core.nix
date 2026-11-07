# claude-plan-feed

spend-watch's feed for a Claude plan (Pro, Max or Ultra): how much of the
plan's five-hour allowance and its weekly allowance is used, and when each
resets. It speaks the [feed contract][contract] on stdout and hears the pace
on stdin, so the watch can warn before a plan stops turns rather than after.
A machine on a Claude plan names it beside its other feeds:

```nix
jitsusama.spend-watch.feeds.claude = {
  label = "Claude";
  command = [ (lib.getExe config.jitsusama.spend-watch.claudePlanFeed) ];
};
```

`claudePlanFeed` is a read-only option of spend-watch's module
([home.nix][home]), as nix-offload gives its own program, so the feed is
reached wherever spend-watch is imported, with no flake output of its own.

## What It Reads

The sign-in is the one pi-claude keeps ([claude.sh][pi-claude]): Claude
Code's `.credentials.json` in its config folder, which is the feed's
argument, else `PI_CLAUDE_CONFIG_DIR`, else `~/.claude-personal` where it
exists, else `~/.claude`. Nothing about it is in Nix. The file holds
`claudeAiOauth`, with `accessToken`, `refreshToken`, `expiresAt` in
milliseconds, `scopes`, `subscriptionType` (such as `pro`) and
`rateLimitTier`. Claude Code keeps it in that file only on Linux; on macOS
it's in the keychain, which this doesn't read, as spend-watch runs only on
Linux too.

The feed reads only the access token, and never renews it. Claude Code's
refresh tokens are good for one use, so a renewal here would leave Claude
Code holding a spent one and sign it out. While the access token has
expired, the feed says the plan is stalled and asks nothing; Claude Code
renews it the next time it runs.

With the token, it asks the OAuth usage endpoint Claude Code's `/usage`
asks:

```text
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <access token>
anthropic-beta: oauth-2025-04-20
```

Its answer holds a window for each allowance, `five_hour` and `seven_day`,
each with a `utilization` in percent and a `resets_at` instant, beside
windows for single models, extra usage and a list of `limits` the feed
doesn't read. A plan without a window answers it as null: Ultra, once Max
20x, has had no five-hour window since the end of October 2026, and its
answer gives `"five_hour": null`.

The endpoint isn't documented and may change. Its name, its header and its
shape are known from tools that read it in the open:

- [Claude Code's issue 99231][99231] asks for an official endpoint, and
  says community tools read `/api/oauth/usage` with the token in
  `~/.claude/.credentials.json`.
- CodexBar's [Claude notes][codexbar-notes] give the request, its beta
  header, the `user:profile` scope it needs, and the plan's name from
  `subscriptionType`; its [fetcher][codexbar-fetcher] reads the windows'
  `utilization` and `resets_at` and treats a 429 with `Retry-After`, five
  minutes when none is given; and [its tests][codexbar-tests] keep a
  captured Max 20x answer with `"five_hour": null`.
- [clawdmeter-wifi's issue 10][clawdmeter] saw a plan's five-hour window
  dropped in early November 2026, and [Claude Code's issue 105240][105240]
  calls Ultra the plan formerly Max 20x.
- Claude Code's [authentication docs][auth] say where the sign-in is kept on
  each system, and that `CLAUDE_CONFIG_DIR` moves it.

## What It Says

Each read writes a `cap` for each window the answer gives, `five-hours` and
`week`, labelled with the plan's name ("Claude Pro five-hour allowance"):
`unit` `share`, `used` from 0 to 1, `resets`, `windowHours` (5 and 168),
`warn` at 0.8, `at`, and `state`, which is `refused` once all of it is used,
`notified` past the warning and `ok` below. A window the answer doesn't
give, or whose reset has passed, isn't said. Then a `health` for the one
source, `plan`, and a `read` of what asking cost. As it starts it offers
an `action`, `sign-in`, running `pi-claude login` in a terminal.

| The sign-in or the answer              | Health       |
| -------------------------------------- | ------------ |
| an answer with a window in it          | `fine`       |
| no `.credentials.json`, or no plan in it | `optional`, with `sign-in` |
| an expired access token                | `stalled`, nothing asked |
| a 429                                  | `stalled`, nothing asked until it's over |
| a 401                                  | `signed-out`, with `sign-in` |
| no `user:profile` scope                | `error`, nothing asked |
| no window it can read, or not JSON     | `error`, and no cap |
| any other status, or no answer         | `error`      |

It reads at the watch's pace: at once as it starts, about every minute while
pi's logs grow, every ten minutes while they don't, and at once on a
refresh. A failure doubles the wait each time, up to an hour, and a 429's
wait is kept however soon a refresh comes.

[tests/claude-plan.jsonl][example] is what it writes for a Pro plan's
recorded answer, and the tests hold its output to it. `spend-watch contract
check` refuses it until spend-watch reads caps of a share.

## Other Sources

Two more say the same figures, and neither is built:

- Every response to a request on the plan carries
  `anthropic-ratelimit-unified-5h-utilization`, `-5h-reset`, `-5h-status`
  and the same for `7d`, as a fraction and epoch seconds, with
  `-status`, `-representative-claim` and `-overage-*` beside them
  ([meneerhenk's issue 351][meneerhenk] keeps a captured 429). Reading
  them means a model request of its own, or a proxy in front of Claude Code.
- Claude Code gives its [status line][statusline] `rate_limits.five_hour` and
  `rate_limits.seven_day`, each `used_percentage` and `resets_at`, but only
  after a response in a running session, and drops a window once it resets.
  Claude Code has no command that prints them.

[contract]: ../../contract/README.md
[home]: ../../home.nix
[pi-claude]: ../../../pi/claude.sh
[example]: tests/claude-plan.jsonl
[99231]: https://github.com/anthropics/claude-code/issues/99231
[105240]: https://github.com/anthropics/claude-code/issues/105240
[codexbar-notes]: https://github.com/steipete/CodexBar/blob/main/docs/claude.md#oauth-api-preferred
[codexbar-fetcher]: https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeOAuth/ClaudeOAuthUsageFetcher.swift
[codexbar-tests]: https://github.com/steipete/CodexBar/blob/main/Tests/CodexBarTests/ClaudeUsageTests.swift
[clawdmeter]: https://github.com/alasdaircs/clawdmeter-wifi/issues/10
[meneerhenk]: https://github.com/StephanMeijer/meneerhenk/issues/351
[auth]: https://code.claude.com/docs/en/authentication
[statusline]: https://code.claude.com/docs/en/statusline
