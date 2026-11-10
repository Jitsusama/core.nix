# Slack's Stand-Ins

What the shell stands beside SlackUnreads.qml, so `tests/read-there.qml` runs it
in offscreen Quickshell without a daemon or the shell's notices: Slack's daemon,
what it holds, and the notices it reads. Each file opens saying what it stands in
for, and the check that lays them out is `slack-reads-a-notice-read-there-once`
in `../../checks.nix`.
