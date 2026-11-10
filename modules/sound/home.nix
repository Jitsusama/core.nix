# The sound daemon: the sound server's outputs and inputs, each saying
# how loud it's set, whether it's muted and whether it's chosen, held
# ready over $XDG_RUNTIME_DIR/daemons/sound.sock in the shell's daemon
# interface, for the launcher's Sound (src/main.rs says how). The shell
# imports it, since that's what it holds.
{ lib, pkgs, ... }:
let
  sound = pkgs.callPackage ./package.nix { };
in
{
  # It isn't in background.slice: what it holds is what the launcher
  # shows as Joel opens it. It's part of the graphical session, as the
  # sound server Joel hears is.
  systemd.user.services.sound = {
    Unit = {
      Description = "Holds the sound server's devices ready for the shell";
      PartOf = [ "graphical-session.target" ];
      After = [
        "graphical-session.target"
        "sound.socket"
        "pipewire.service"
      ];
      # Restarted after a switch reloads the units, it comes back as the
      # new one.
      X-SwitchMethod = "restart";
      Requires = [ "sound.socket" ];
    };
    Service = {
      ExecStart = "${lib.getExe sound} %t/daemons/sound.sock";
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };

  # Its socket, held by systemd whether or not the daemon runs, so the
  # shell asking while it starts waits on it rather than finding none.
  systemd.user.sockets.sound = {
    Unit.Description = "Where the shell asks the sound daemon";
    Socket = {
      ListenStream = "%t/daemons/sound.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };
}
