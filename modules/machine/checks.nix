# The checks of the machine daemon, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
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
# The machine daemon reads Linux's /proc and /sys, and builds only there.
onLinux {
  # Its tests run as it builds, over a machine of its own in a fixture
  # tree: hello on the socket systemd hands it, every record of a watch's
  # kinds at once and its rates as of now a moment later, a disk's rates,
  # sensors, link and file systems, a disk taken out said gone, programs
  # ranked, nothing read once nothing is watched, and an ask or kind it
  # doesn't know refused.
  machine-holds-the-machine-ready = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the machine daemon, its tests included.
  machine-passes-clippy = passesClippy "machine-passes-clippy" (pkgs.callPackage ./package.nix { });
  # The shell brings the daemon, on the socket the shell's daemons share,
  # held by systemd whether or not the daemon runs.
  machine-runs-beside-the-shell =
    let
      home = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.quickshell ]; } ]);
      service = home.systemd.user.services.machine;
    in
    holds "machine-runs-beside-the-shell"
      "The machine daemon doesn't run beside the shell on its socket, held by systemd."
      (
        # Home Manager keeps a unit's ExecStart as a list of its lines.
        lib.hasInfix "%t/daemons/machine.sock" (
          lib.concatStringsSep " " (lib.toList service.Service.ExecStart)
        )
        && service.Unit.X-SwitchMethod == "restart"
        && builtins.elem "machine.socket" service.Unit.Requires
        && home.systemd.user.sockets.machine.Socket.ListenStream == "%t/daemons/machine.sock"
      );
}
