# The checks of pi-mesh-reader, which reads each folder of pi's mesh once
# for every pi process that asks, beside pi's own in ../checks.nix.
{
  account,
  holds,
  lib,
  macWorkstation,
  onLinux,
  passesClippy,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # The reader is started by the first pi process that asks, on a socket in
  # a folder only the account reaches, reads only under ~/.local/state/pi,
  # is tagged in the journal, runs outside agents.slice with a bound of its
  # own, and comes back after a failure. A Mac gets none of it.
  pi-mesh-reader-answers-only-the-account =
    let
      home = account workstation;
      mac = account macWorkstation;
      socket = home.systemd.user.sockets.pi-mesh-reader.Socket;
      reader = home.systemd.user.services.pi-mesh-reader;
      start = lib.head (lib.toList reader.Service.ExecStart);
    in
    holds "pi-mesh-reader-answers-only-the-account"
      "pi's mesh reader listens elsewhere than %t/pi-mesh-reader/reader.sock or to more than the account, isn't started by its socket, reads beyond ~/.local/state/pi, isn't tagged in the journal, runs in agents.slice or without a bound of its own, stays down after a failure, or a Mac gets it."
      (
        lib.head (lib.toList socket.ListenStream) == "%t/pi-mesh-reader/reader.sock"
        && lib.head (lib.toList socket.SocketMode) == "0600"
        && lib.head (lib.toList socket.DirectoryMode) == "0700"
        && reader.Unit.Requires == [ "pi-mesh-reader.socket" ]
        && lib.hasSuffix "/bin/pi-mesh-reader --root %h/.local/state/pi" start
        && lib.length (lib.splitString "--root" start) == 2
        && reader.Service.SyslogIdentifier == "pi-mesh-reader"
        && !(reader.Service ? Slice)
        && reader.Service.MemoryMax == "512M"
        && reader.Service.Restart == "on-failure"
        && !(mac.systemd.user.sockets ? pi-mesh-reader)
        && !(mac.systemd.user.services ? pi-mesh-reader)
      );

  # pi-mesh-reader builds with its tests run (src/*.rs, tests/serves.rs): a
  # client asking for a folder under a root is told what it holds, then
  # ready, then each file put or gone, once, however many clients ask, all
  # from one follower; a folder outside every root, through a link or not,
  # or not there, is refused; a folder moved away and made again is
  # followed again, saying what went and came; the last client going lets
  # the folder go; and a client that falls behind is let go without
  # holding the rest.
  pi-mesh-reader-reads-once-for-all = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in pi-mesh-reader, its tests included.
  pi-mesh-reader-passes-clippy = passesClippy "pi-mesh-reader-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
