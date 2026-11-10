# The checks of the windows daemon, which checks.nix gathers with every
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
# It reads niri and Linux's inotify, and builds only there.
onLinux {
  # Its tests run as it builds, over niri's events as its stream says
  # them and over a stand-in for niri's socket: each window named by its
  # program and placed as its row says it, a retitle telling only its
  # window and those it reads alike with, a focus moving only the two it
  # moved between, a resize or the launcher taking the keys nothing, a
  # window closed gone, an entry added naming its windows' program, niri
  # gone and back describing the desktop again, and an ask or kind it
  # doesn't know refused.
  windows-hears-niris-windows = daemon;
  # clippy finds nothing in the windows daemon, its tests included.
  windows-passes-clippy = passesClippy "windows-passes-clippy" daemon;
  # The shell brings the daemon, on the socket the shell's daemons share,
  # held by systemd whether or not the daemon runs.
  windows-runs-beside-the-shell =
    let
      shell = {
        home-manager.sharedModules = [ self.homeModules.quickshell ];
      };
      home = account (bareNixos [ shell ]);
      service = home.systemd.user.services.windows;
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
    in
    holds "windows-runs-beside-the-shell"
      "The windows daemon doesn't run beside the shell on its socket."
      (
        lib.hasInfix "%t/daemons/windows.sock" started
        && service.Unit.X-SwitchMethod == "restart"
        && builtins.elem "windows.socket" service.Unit.Requires
        && home.systemd.user.sockets.windows.Socket.ListenStream == "%t/daemons/windows.sock"
      );
}
