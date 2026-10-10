# Joel types Colemak Mod-DH. His own keyboard lays it out in its firmware and
# sends US key codes, as a YubiKey does, but a laptop's keyboard needs the
# layout from the system. keyd remaps the built-in keyboard alone, below the
# compositor, so the layout holds everywhere: the console, the session, and
# the systemd initrd, where the disk's PIN is typed.
{ config, lib, ... }:
{
  imports = [ ../keyboard/nixos.nix ];

  services.keyd = {
    enable = true;
    keyboards.built-in = {
      ids = config.jitsusama.keyboard.builtIn;
      extraConfig = builtins.readFile ./colemak-dh.conf;
    };
  };

  # keyd leaves on SIGTERM by calling exit with the signal's number, so a
  # stop ends in status 15, which systemd would otherwise count as a failure
  # at every shutdown and every switch to the running system.
  systemd.services.keyd.serviceConfig.SuccessExitStatus = 15;

  # The same keyd in the initrd, reading the same file, so the layout is the
  # same when the disk asks for its PIN. It stops at the switch to the running
  # system, whose keyd takes over. evdev gives it the keyboard's events and
  # uinput the keyboard it types through; the console's own keyboard needs
  # neither, so the initrd doesn't load them unasked.
  boot.initrd.kernelModules = [
    "evdev"
    "uinput"
  ];
  boot.initrd.systemd = {
    contents."/etc/keyd/built-in.conf".source = config.environment.etc."keyd/built-in.conf".source;
    storePaths = [ (lib.getExe config.services.keyd.package) ];
    services.keyd = {
      description = "Remaps the built-in keyboard before the disk is unlocked";
      wantedBy = [ "sysinit.target" ];
      after = [ "systemd-modules-load.service" ];
      before = [ "cryptsetup-pre.target" ];
      wants = [ "cryptsetup-pre.target" ];
      unitConfig.DefaultDependencies = false;
      serviceConfig = {
        ExecStart = lib.getExe config.services.keyd.package;
        SuccessExitStatus = 15;
      };
    };
  };
}
