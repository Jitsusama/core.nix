# The checks of the programs daemon, which checks.nix gathers with every
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
let
  daemon = pkgs.callPackage ./package.nix { };
in
# It reads Linux's inotify, and builds only there.
onLinux {
  # Its tests run as it builds, over data folders and sources of its own:
  # hello on the socket systemd hands it, an entry whole from the nearest
  # folder, an entry changed said again and one removed said gone, a list
  # read as its watch begins and as its file changes and not while nobody
  # watches, one whose command fails heard empty, what a program has open
  # read as it's asked, a data folder followed through each link a switch
  # replaces, and an ask or kind it doesn't know refused.
  programs-holds-whats-installed = daemon;
  # clippy finds nothing in the programs daemon, its tests included.
  programs-passes-clippy = passesClippy "programs-passes-clippy" daemon;
  # The shell brings the daemon, on the socket the shell's daemons share,
  # held by systemd whether or not the daemon runs, given every source a
  # program's module lists.
  programs-runs-beside-the-shell =
    let
      shell = {
        home-manager.sharedModules = [ self.homeModules.quickshell ];
      };
      home = account (bareNixos [ shell ]);
      service = home.systemd.user.services.programs;
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
    in
    holds "programs-runs-beside-the-shell"
      "The programs daemon doesn't run beside the shell on its socket, given its sources."
      (
        lib.hasInfix "%t/daemons/programs.sock" started
        && lib.hasInfix "programs-sources.json" started
        && service.Unit.X-SwitchMethod == "restart"
        && builtins.elem "programs.socket" service.Unit.Requires
        && home.systemd.user.sockets.programs.Socket.ListenStream == "%t/daemons/programs.sock"
      );
}
