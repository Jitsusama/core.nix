# The Feed's Tests

Each file proves one part of the feed through the library's own surface:
`feed.rs` one read, `schedule.rs` when the next is due, `heard.rs` what the
watch writes on stdin, and `folder.rs` which config folder is read.
`claude-plan.jsonl` is what the feed writes for a Pro plan's answer, as an
example of its contract.

- `answers/`: the usage endpoint's answers, shaped as the captures
  [README.md](../README.md) cites are: a Pro plan's two windows, Ultra's week
  alone, windows used up, and one with no window that reads.
- `logins/`: Claude Code's `.credentials.json` as it keeps a sign-in, with
  tokens made for the tests: a Pro plan's, Ultra's, an expired one, one
  without the scope usage needs, one holding only an MCP server's, and one
  cut off mid-write.
