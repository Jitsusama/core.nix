# The listener's command, which darwin.nix runs and the VM test runs too, so
# what the test proves is what a Mac runs. socat keeps each datagram from an
# address in `from` and drops the rest unread, appending what it keeps to
# `log` as it came; it won't follow a link left where the log goes.
{
  socat,
  port,
  from,
  log,
}:
[
  socat
  "-u"
  "UDP-RECV:${toString port},range=${from}"
  "OPEN:${log},creat,append,o-nofollow"
]
