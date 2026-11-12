# The checks of the secret-asks daemon, which checks.nix gathers with
# every other folder's; its opening comment says what each kind of check
# proves.
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
# It serves the shell, which runs only on Linux.
onLinux {
  # The daemon's tests run as it builds: a yes releases the one item to
  # the checker alone, which says its length and never the value; an ask
  # for any other item reads nothing and draws no row; a yes for a line
  # whose ask or tree changed since it was drawn is refused and the line
  # drawn again; a yes while locked reads nothing; and a no refuses.
  secret-asks-keeps-the-item-to-its-yes = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the secret-asks daemon, its tests included.
  secret-asks-passes-clippy = passesClippy "secret-asks-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
  # The daemon runs on its socket with the item the machine names,
  # reading it through op's wrapper, shown by its name, and a switch restarts
  # it.
  secret-asks-runs-with-its-item =
    let
      item = "op://Private/Release Test/password";
      home = account (bareNixos [
        {
          home-manager.users.someone = {
            imports = [ self.homeModules.secret-asks ];
            jitsusama.secret-asks.item = item;
          };
        }
      ]);
      service = home.systemd.user.services.secret-asks;
      socket = home.systemd.user.sockets.secret-asks;
      # Home Manager keeps a unit's ExecStart as a list of its lines.
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
    in
    holds "secret-asks-runs-with-its-item"
      "The secret-asks daemon lacks its item, op's wrapper or its socket, or outlives a switch."
      (
        lib.hasPrefix "@" started
        && lib.hasInfix " secret-asks serve " started
        && lib.hasInfix "--item ${lib.escapeShellArg item}" started
        && lib.hasInfix "--op /run/wrappers/bin/op" started
        && lib.hasInfix "--socket %t/daemons/secret-asks.sock" started
        && !(lib.hasInfix "--account" started)
        && socket.Socket.ListenStream == "%t/daemons/secret-asks.sock"
        && service.Unit.Requires == [ "secret-asks.socket" ]
        && service.Unit.X-SwitchMethod == "restart"
      );
}
