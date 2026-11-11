# The checks of the sound daemon, which checks.nix gathers with every
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
# It hears PipeWire, and builds only on Linux.
onLinux {
  # Its tests run as it builds, over what PipeWire says and with no sound
  # server at all: outputs and inputs made devices and a program's stream
  # left out, a volume said as the shell shows it and a mute alone keeping
  # it, the chosen ones those the default metadata names, a node gone let
  # go of, a card saying what it is and whether anything is plugged in
  # where each device plays, each channel's volume and the mute read
  # from a Props and a card's routes from its EnumRoute, Sound's rows
  # named, ordered, worn and worded as the launcher's, an empty jack
  # listed only while chosen, a volume moving sending its own row alone, hello
  # saying PipeWire isn't answering, a list and a watch answering no
  # devices or rows rather than waiting, and an ask or kind it doesn't know
  # refused.
  sound-hears-pipewires-devices = daemon;
  # clippy finds nothing in the sound daemon, its tests included.
  sound-passes-clippy = passesClippy "sound-passes-clippy" daemon;
  # The shell brings the daemon, on the socket the shell's daemons share,
  # held by systemd whether or not the daemon runs.
  sound-runs-beside-the-shell =
    let
      shell = {
        home-manager.sharedModules = [ self.homeModules.quickshell ];
      };
      home = account (bareNixos [ shell ]);
      service = home.systemd.user.services.sound;
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
    in
    holds "sound-runs-beside-the-shell" "The sound daemon doesn't run beside the shell on its socket." (
      lib.hasInfix "%t/daemons/sound.sock" started
      && service.Unit.X-SwitchMethod == "restart"
      && builtins.elem "sound.socket" service.Unit.Requires
      && home.systemd.user.sockets.sound.Socket.ListenStream == "%t/daemons/sound.sock"
    );
}
