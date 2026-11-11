# The checks of warp, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# Its checks build where every other Rust program's do; warp itself builds
# on a Mac too, where WARP's app puts warp-cli.
onLinux {
  # warp's tests run as it builds: every answer in the contract reads into
  # exactly the record warp.json holds, a reason given with figures keeps
  # them, an endpoint given as no address is none and a figure left out is
  # unknown rather than zero; no warp-cli, a service not answering or one
  # that hangs, and a status refused or garbled are each told apart, said
  # in a line and ended with a code of their own, and with --json as a
  # record; a part besides the status that warp-cli won't give is unknown
  # and the rest still reads; an override in force says how long it has
  # left; each page matches its golden one (src/golden/), connected,
  # degraded and disconnected; and connect and disconnect leave a switch
  # the organisation holds as it is, asking warp-cli nothing, say WARP
  # already as asked, say when WARP always on comes back, and say warp-cli
  # refusing in its words, each ending with its own code; and serving the
  # shell, a status or reason a listening warp-cli tells, other than the
  # last, has the link read at once, and a figure moving or a DNS log
  # doesn't.
  warp-reads-what-warp-cli-says = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in warp, its tests included.
  warp-passes-clippy = passesClippy "warp-passes-clippy" (pkgs.callPackage ./package.nix { });
}
