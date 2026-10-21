# The firmware's own settings, the ones its setup screen shows, held to those
# the machine declares. Its hardware's module says which, since only that
# knows what its firmware offers. `firmware-settings check` names what
# differs, and `run0 firmware-settings apply` writes it, asking for the
# firmware's admin password, which nothing keeps. A check fails at boot
# when the firmware holds anything else. The hardware's module also says how
# a person reaches its firmware's screens, which the guide tells them when a
# stick is written for the machine: how to start it from a stick, and how to
# clear its Secure Boot keys.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  settings = config.jitsusama.firmware.settings;
  firmwareSettings = pkgs.callPackage ./package.nix { };
in
{
  options.jitsusama.firmware.settings = lib.mkOption {
    type = lib.types.attrsOf lib.types.str;
    default = { };
    example = {
      PrimaryBattChargeCfg = "Custom";
    };
    description = ''
      Firmware settings by the names the kernel's firmware-attributes class
      gives them, under /sys/class/firmware-attributes/*/attributes, with
      the values they should hold.
    '';
  };

  options.jitsusama.firmware.startFromStick = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "Press F12 as it starts, and pick the stick.";
    description = "How a person starts the machine from a USB stick, said as they'd do it.";
  };

  options.jitsusama.firmware.clearSecureBootKeys = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "Press F2 as it starts; under Secure Boot, delete all keys.";
    description = ''
      How a person clears the firmware's Secure Boot keys, putting it in
      setup mode, said as they'd do it.
    '';
  };

  config = lib.mkIf (settings != { }) {
    environment.etc."firmware-settings".text = lib.concatLines (
      lib.mapAttrsToList (name: value: "${name}=${value}") settings
    );
    environment.systemPackages = [ firmwareSettings ];

    # Started as the firmware's interface appears. Its driver announces it
    # before making the settings in it, so the check waits for them.
    systemd.services.firmware-settings-match = {
      description = "Check the firmware holds its declared settings";
      environment.FIRMWARE_SETTINGS_WAIT = "30";
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${firmwareSettings}/bin/firmware-settings check";
      };
    };
    services.udev.extraRules = ''
      SUBSYSTEM=="firmware-attributes", ACTION=="add", TAG+="systemd", ENV{SYSTEMD_WANTS}+="firmware-settings-match.service"
    '';
  };
}
