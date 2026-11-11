# The checks of the to-do daemon, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
# The daemon reads Linux's inotify, /proc and sysfs, and builds only there.
onLinux {
  # The daemon's tests run as it builds: each signal is read from its
  # words and a relative file is none; a to-do is keyed by its quest and
  # id and judged only where something here can see it; a boot, a switch,
  # a switch to a commit or past it, a sign-in, a device and a file are
  # each heard, a boot or a switch only after the to-do was raised; a
  # failing check waits longer each time, to ten minutes; a check says its
  # exit and its last line, has the session's variables, and one that
  # hangs is stopped with all it started, and one in time leaves nothing it
  # started; the user manager's environment is read with its quoting; an
  # owner is open only where its sandbox is said to be none; a device name
  # of no letters matches none; the verdicts say every figure, read back what was done and
  # are the same text for the same verdicts; and run over a folder of
  # to-dos, a passing check is done and stays done though it fails after
  # and across a restart, a failing one waits, a hanging one holds up no
  # other, a file is done as it comes, a boot makes one ready, a to-do
  # gone takes its verdict, a check runs only for an owner outside every
  # sandbox, and a check finds what the session's PATH holds; an ask names
  # its kinds and leaves out another daemon's keys; each to-do is in the
  # list For You puts it in, by its verdict, its owner's word and Joel's,
  # put off only while it says what it said; each look is held whole and
  # counted by its state; and over its socket a watch is told each to-do,
  # look and word of Joel's as it changes, and a to-do gone as it goes;
  # each to-do is worded for For You from the launcher's words, its note's
  # parts, its drawer's facts in order and its acts asking the daemon; and
  # Joel's word, asked as an act, is kept beside what he said before, and
  # a file that can't be read is left as it is.
  to-do-clears-each-to-do-by-itself = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the to-do daemon, its tests included.
  to-do-passes-clippy = passesClippy "to-do-passes-clippy" (pkgs.callPackage ./package.nix { });
  # The daemon runs in each account with its own git, journalctl and
  # systemctl named, its checks on the user manager's environment as it is
  # when each runs, its socket systemd's and the looks and Joel's word read
  # beside it, and a switch restarts it.
  to-do-runs-in-each-account =
    let
      home = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.to-do ]; } ]);
      service = home.systemd.user.services.to-do;
      socket = home.systemd.user.sockets.to-do;
      # Home Manager keeps a unit's ExecStart as a list of its lines, and
      # gives every unit an Environment, empty unless something sets it.
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
    in
    holds "to-do-runs-in-each-account"
      "The to-do daemon lacks its git, journalctl, systemctl or socket, or outlives a switch."
      (
        lib.hasInfix "--git /nix/store/" started
        && lib.hasInfix "--journalctl /nix/store/" started
        && lib.hasInfix "--systemctl /nix/store/" started
        && lib.hasInfix "--socket %t/daemons/to-do.sock" started
        && lib.hasInfix "/pi/agentic-harness.pi/looks" started
        && lib.hasInfix "/mission/settled.json" started
        && socket.Socket.ListenStream == "%t/daemons/to-do.sock"
        && service.Unit.Requires == [ "to-do.socket" ]
        && service.Service.Environment == [ ]
        && service.Unit.X-SwitchMethod == "restart"
      );
}
