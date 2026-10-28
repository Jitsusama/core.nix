# A paired Bluetooth keyboard types at the disk prompt, as the built-in one
# does. The kernel's Bluetooth drivers, BlueZ and D-Bus run in the systemd
# initrd, and BlueZ has the machine's pairings there before the disk is
# asked for. Each hardware module names its controller's firmware in
# boot.initrd.extraFirmwarePaths, since the drivers don't list it.
#
# The pairings are the keyboard's keys: anyone holding one can type as the
# keyboard. So they reach the initrd only sealed to the TPM, bound to
# Secure Boot's state, from the ESP's \loader\credentials, which the boot
# stub hands to the initrd. seal.sh says when they're sealed and why to
# that. With none sealed, or a seal that won't open, the
# prompt asks as it always has, on the built-in keyboard, waiting for
# nothing.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  bluez = config.hardware.bluetooth.package;
  initrd = config.boot.initrd.systemd;
  dbus = config.services.dbus;

  seal = pkgs.writeShellApplication {
    name = "bluetooth-pairing-seal";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.findutils
      pkgs.gnugrep
      pkgs.gnutar
      config.systemd.package
    ];
    text = builtins.readFile ./seal.sh;
  };

  sealEnvironment.ESP = config.boot.loader.efi.efiSysMountPoint;
in
{
  imports = [ ../bluetooth/nixos.nix ];

  # The controller's USB driver, which brings in each chip's own, and the
  # two ways a keyboard's input reaches the kernel: uhid, which BlueZ uses
  # for both classic and Low Energy keyboards, and hidp, the kernel's own
  # for classic ones. udev loads btusb as the controller appears; the other
  # two are opened, not probed, so they're loaded outright.
  boot.initrd.availableKernelModules = [
    "btusb"
    "hid_generic"
  ];
  boot.initrd.kernelModules = [
    "hidp"
    "uhid"
  ];

  boot.initrd.systemd = {
    # nixpkgs' initrd bus allows only systemd and itself; BlueZ's policy
    # joins them, so bluetoothd may own its name there.
    dbus.enable = true;
    contents."/etc/dbus-1".source = lib.mkForce (
      pkgs.makeDBusConf.override {
        inherit (dbus) apparmor;
        dbus = dbus.dbusPackage;
        suidHelper = "/bin/false";
        serviceDirectories = [
          dbus.dbusPackage
          initrd.package
          bluez
        ];
      }
    );
    contents."/etc/bluetooth/main.conf".source = config.environment.etc."bluetooth/main.conf".source;
    storePaths = [
      "${bluez}/libexec/bluetooth/bluetoothd"
      "${bluez}/share/dbus-1/system.d"
      (lib.getExe pkgs.gnutar)
    ];

    # The sealed pairings, opened into BlueZ's state. systemd opens the
    # credential itself, and only with the TPM and the policy the disk's key
    # needs; it's skipped when there's none.
    services.bluetooth-pairing = {
      description = "Opens the sealed Bluetooth pairings for the disk prompt";
      before = [ "bluetooth.service" ];
      unitConfig = {
        DefaultDependencies = false;
        ConditionCredential = "bluetooth-pairing";
      };
      serviceConfig = {
        Type = "oneshot";
        ImportCredential = "bluetooth-pairing";
        StateDirectory = "bluetooth";
        StateDirectoryMode = "0700";
        ExecStart = "${lib.getExe pkgs.gnutar} --extract --file=%d/bluetooth-pairing --directory=%S/bluetooth";
      };
    };

    # bluetoothd, started before the disk is asked for so a keyboard can
    # reconnect while the prompt waits. It's only ordered before the prompt,
    # never needed by it, so a bluetoothd that fails keeps nothing waiting.
    # It stops at the switch to the running system, whose own takes over.
    services.bluetooth = {
      description = "Bluetooth for the disk prompt";
      wantedBy = [ "sysinit.target" ];
      wants = [
        "bluetooth-pairing.service"
        "dbus.socket"
        "cryptsetup-pre.target"
      ];
      after = [
        "bluetooth-pairing.service"
        "dbus.socket"
        "systemd-udevd.service"
      ];
      before = [ "cryptsetup-pre.target" ];
      unitConfig.DefaultDependencies = false;
      serviceConfig = {
        Type = "dbus";
        BusName = "org.bluez";
        ExecStart = "${bluez}/libexec/bluetooth/bluetoothd --configfile=/etc/bluetooth/main.conf";
      };
    };
  };

  # Sealed again whenever the pairings change: every few minutes, cheaply,
  # since it seals only what's new, and once more at shutdown, after BlueZ
  # has written its last.
  systemd.services.bluetooth-pairing-seal = {
    description = "Seals the Bluetooth pairings for the disk prompt";
    after = [ "bluetooth.service" ];
    unitConfig.RequiresMountsFor = [ config.boot.loader.efi.efiSysMountPoint ];
    environment = sealEnvironment;
    serviceConfig = {
      Type = "oneshot";
      ExecStart = lib.getExe seal;
      StateDirectory = "bluetooth-pairing-seal";
      PrivateTmp = true;
    };
  };
  systemd.timers.bluetooth-pairing-seal = {
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnBootSec = "1min";
      OnUnitActiveSec = "2min";
    };
  };
  systemd.services.bluetooth-pairing-seal-at-shutdown = {
    description = "Seals the Bluetooth pairings for the disk prompt at shutdown";
    wantedBy = [ "multi-user.target" ];
    # Stopped, and so sealing, before bluetooth.service stops and before the
    # ESP is unmounted.
    after = [ "bluetooth.service" ];
    unitConfig.RequiresMountsFor = [ config.boot.loader.efi.efiSysMountPoint ];
    environment = sealEnvironment;
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStop = lib.getExe seal;
      StateDirectory = "bluetooth-pairing-seal";
      PrivateTmp = true;
    };
  };
}
