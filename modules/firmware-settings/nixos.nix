# The firmware's own settings, the ones its setup screen shows, held to those
# the machine declares. Its hardware's module says which, since only that
# knows what its firmware offers. `firmware-settings check` names what
# differs, and `run0 firmware-settings apply` writes it, asking for the
# firmware's admin password, which nothing keeps. A check fails at boot
# when the firmware holds anything else, except a setting waiting for a
# person at the setup screen, which it names. The hardware's module also says how
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
  waiting = config.jitsusama.firmware.waiting;
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

  options.jitsusama.firmware.waiting = lib.mkOption {
    type = lib.types.attrsOf lib.types.str;
    default = { };
    example = {
      AllowBiosDowngrade = "someone to turn it off at the firmware's setup screen";
    };
    description = ''
      Declared settings a person has yet to set at the firmware's setup
      screen, each with what it waits for. The check names them without
      failing, so a boot before then isn't marked as broken, and apply
      writes them as any other.
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

  options.jitsusama.firmware.secureBoot = lib.mkOption {
    type = lib.types.bool;
    default = true;
    example = false;
    description = ''
      Whether the firmware has a Secure Boot that takes the machine's own
      keys. The guide makes, seals and signs with them only where it does.
    '';
  };

  options.jitsusama.firmware.tpm = lib.mkOption {
    type = lib.types.bool;
    default = true;
    example = false;
    description = ''
      Whether the machine has a TPM 2.0 for the system to keep keys in. The
      guide seals the disk to it, and makes the accounts' keys in it, only
      where it does.
    '';
  };

  config = lib.mkIf (settings != { }) {
    assertions = [
      {
        assertion = lib.all (name: settings ? ${name}) (lib.attrNames waiting);
        message = "jitsusama.firmware.waiting names a setting that isn't declared";
      }
    ];
    environment.etc."firmware-settings".text = lib.concatLines (
      lib.mapAttrsToList (name: value: "${name}=${value}") settings
    );
    environment.etc."firmware-settings-waiting".text = lib.concatLines (
      lib.mapAttrsToList (name: what: "${name}=${what}") waiting
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
