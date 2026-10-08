# Joel types Colemak Mod-DH on a laptop's own keyboard and US QWERTY on any
# other, a YubiKey included. keyd remaps the built-in keyboard alone, below
# the compositor, so the layout holds everywhere: the console, the session,
# and the systemd initrd, where the disk's PIN is typed.
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
      serviceConfig.ExecStart = lib.getExe config.services.keyd.package;
    };
  };
}
