# The checks of pi-github-door, the door pi's fenced commands reach GitHub
# through, beside pi's own in ../checks.nix.
{
  account,
  holds,
  lib,
  macWorkstation,
  passesClippy,
  pkgs,
  workstation,
  ...
}:
{
  # The door runs on every Linux machine, since the fence does, started by
  # the first fenced command that asks for a session, on a control socket
  # in a folder only the account can reach, which the fence never lets in;
  # it reads the token with gh, and its sessions' folders sit beside the
  # control socket; a Mac gets none.
  pi-github-door-runs-beside-the-fence =
    let
      home = account workstation;
      mac = account macWorkstation;
      socket = home.systemd.user.sockets.pi-github-door.Socket;
      start = lib.head (lib.toList home.systemd.user.services.pi-github-door.Service.ExecStart);
    in
    holds "pi-github-door-runs-beside-the-fence"
      "pi's GitHub door doesn't run on a Linux machine, or runs on a Mac, listens elsewhere than %t/pi-github/control.sock or to more than the account, keeps its sessions elsewhere than %t/pi-github/sessions, reads the token with another program than gh, or isn't on the account's PATH for the fence's relay."
      (
        lib.head (lib.toList socket.ListenStream) == "%t/pi-github/control.sock"
        && lib.head (lib.toList socket.SocketMode) == "0600"
        && lib.head (lib.toList socket.DirectoryMode) == "0700"
        && lib.hasInfix "/bin/pi-github-door serve" start
        && lib.hasInfix "--sessions %t/pi-github/sessions" start
        && lib.hasInfix "--gh /nix/store/" start
        && lib.hasInfix "/bin/gh " "${start} "
        && lib.any (package: (package.pname or "") == "pi-github-door") home.home.packages
        && !(mac.systemd.user.services ? pi-github-door)
        && !(mac.systemd.user.sockets ? pi-github-door)
      );

  # pi-github-door builds with its tests run (tests/door.rs, and the
  # judge's own): a read sent on with the person's token in the
  # placeholder's place, and every refused call never reaching GitHub,
  # naming its route: a merge, a push, a pull request opened, any
  # credential, a repository's settings, protection and rulesets written,
  # and a mutation off the list, judged by its fields through aliases and
  # fragments, never its name, with a mutation over GET, a batch, a stored
  # query or a document that won't parse refused too; a credential other
  # than the placeholder refused; the token read again on a 401 and tried
  # once; git's fetch passed with the token; another host tunnelled
  # untouched and unsaid; one session's bundle not trusting another's
  # door; a session opened for the harness, the one asked for least lately
  # given back once 256 are open; and the relay sending GitHub's hosts to
  # the door and the rest straight out, leaving once the process it
  # watches does.
  pi-github-door-lets-only-allowed-calls-through = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in pi-github-door, its tests included.
  pi-github-door-passes-clippy = passesClippy "pi-github-door-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
