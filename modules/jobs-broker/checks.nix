# The checks of the jobs broker, which checks.nix gathers with every other
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
# systemd's user manager, which it asks, runs only on Linux.
onLinux {
  # The broker's tests run as it builds: a scope goes only into agents'
  # own work, for a process already an agent's, named so polkit knows it
  # as one; a quest names only a slice under agents' work; a status is
  # asked only of a unit's name; the kernel names who asked; and over its
  # socket a scope for a process outside agents' work is refused, a status
  # refuses names that aren't units', and a stop, start or restart is an
  # ask it doesn't know.
  jobs-broker-scopes-only-agents-work = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the jobs broker, its tests included.
  jobs-broker-passes-clippy = passesClippy "jobs-broker-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
  # The broker runs on its socket in the daemons' folder, shown by its
  # name, and a switch restarts it.
  jobs-broker-runs-on-its-socket =
    let
      home = account (bareNixos [
        { home-manager.users.someone.imports = [ self.homeModules.jobs-broker ]; }
      ]);
      service = home.systemd.user.services.jobs-broker;
      socket = home.systemd.user.sockets.jobs-broker;
      # Home Manager keeps a unit's ExecStart as a list of its lines.
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
    in
    holds "jobs-broker-runs-on-its-socket"
      "The jobs broker isn't on its socket, isn't shown by its name, or outlives a switch."
      (
        lib.hasPrefix "@" started
        && lib.hasInfix " jobs-broker serve " started
        && lib.hasInfix "--socket %t/daemons/jobs-broker.sock" started
        && socket.Socket.ListenStream == "%t/daemons/jobs-broker.sock"
        && socket.Socket.SocketMode == "0600"
        && service.Unit.Requires == [ "jobs-broker.socket" ]
        && service.Unit.X-SwitchMethod == "restart"
      );
}
