# A paired Bluetooth keyboard types at the disk prompt of a machine
# installed as a laptop is, with Secure Boot and measured boot. The laptop
# pairs with a stand-in keyboard on a second machine, seals the pairing to
# its TPM, and restarts; the keyboard types the passphrase at the prompt,
# and the disk opens. The sealed pairing never holds the link key where the
# ESP can be read, the laptop's address is the one it paired with, and a
# seal that won't open leaves the prompt asking as it always has. The
# script is bluetooth-disk-prompt-vm-test.py.
#
# The two machines' controllers are btvirt's, BlueZ's emulator, run by the
# driver as a TCP server: each machine's second serial port is a QEMU
# socket to it, which btattach makes a controller of. QEMU holds the socket
# across a restart, so the laptop keeps its controller and its address, as
# a laptop's built-in one does.
{ self, pkgs }:
let
  passphrase = "install-passphrase";
  port = 45550;

  # btvirt is built only with BlueZ's tests, which nixpkgs leaves out.
  btvirt = pkgs.bluez.overrideAttrs (old: {
    configureFlags = old.configureFlags ++ [ "--enable-testing" ];
    doCheck = false;
    postInstall = old.postInstall + ''
      install -Dm755 emulator/btvirt $out/bin/btvirt
    '';
  });

  # Each machine's second serial port, ttyS1, to btvirt.
  controller = [
    "-chardev socket,id=bluetooth,host=127.0.0.1,port=${toString port}"
    "-device isa-serial,chardev=bluetooth,index=1"
  ];
  attach = bluez: "${bluez}/bin/btattach --bredr /dev/ttyS1 --protocol h4";

  standIn = pkgs.writeShellApplication {
    name = "bluetooth-keyboard-stand-in";
    text = ''
      exec ${
        pkgs.python3.withPackages (python: [ python.dbus-fast ])
      }/bin/python3 ${./bluetooth-keyboard-stand-in.py} "$@"
    '';
  };
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "bluetooth-disk-prompt";
  globalTimeout = 40 * 60;

  nodes.laptop =
    { config, lib, ... }:
    let
      bluez = config.hardware.bluetooth.package;
    in
    {
      imports = [
        (import ./vm/guide/secure-boot-machine.nix { inherit self pkgs passphrase; })
        self.nixosModules.bluetooth-disk-prompt
      ];
      virtualisation.qemu.options = controller;
      # The driver names each machine by its host name.
      networking.hostName = lib.mkForce "laptop";

      # The controller on the serial port, at the prompt and once the
      # system runs, which a laptop's own controller needs neither of.
      boot.initrd.kernelModules = [ "hci_uart" ];
      boot.initrd.systemd.storePaths = [ "${bluez}/bin/btattach" ];
      boot.initrd.systemd.services.btattach = {
        wantedBy = [ "sysinit.target" ];
        before = [ "bluetooth.service" ];
        after = [ "systemd-udevd.service" ];
        unitConfig.DefaultDependencies = false;
        serviceConfig.ExecStart = attach bluez;
      };
      systemd.services.btattach = {
        wantedBy = [ "multi-user.target" ];
        before = [ "bluetooth.service" ];
        serviceConfig.ExecStart = attach bluez;
      };
    };

  nodes.keyboard =
    { config, ... }:
    {
      # The laptop's disk image is given in the environment, which a
      # machine with a disk of its own would take too.
      virtualisation.diskImage = null;
      virtualisation.qemu.options = controller;

      # BlueZ's input plugin would take the two HID channels the stand-in
      # holds.
      hardware.bluetooth = {
        enable = true;
        powerOnBoot = true;
        disabledPlugins = [ "input" ];
      };
      systemd.services.btattach = {
        wantedBy = [ "multi-user.target" ];
        before = [ "bluetooth.service" ];
        serviceConfig.ExecStart = attach config.hardware.bluetooth.package;
      };
      systemd.services.stand-in-keyboard = {
        wantedBy = [ "multi-user.target" ];
        after = [ "bluetooth.service" ];
        requires = [ "bluetooth.service" ];
        serviceConfig = {
          ExecStart = "${standIn}/bin/bluetooth-keyboard-stand-in serve";
          # Until BlueZ has the controller btattach makes.
          Restart = "on-failure";
          RestartSec = 1;
        };
      };
      environment.systemPackages = [ standIn ];
    };

  tests = ./bluetooth-disk-prompt-vm-test.py;
  values =
    { nodes, ... }:
    let
      inherit (nodes.laptop.system.build) diskoImages toplevel;
    in
    pkgs.writeText "bluetooth-disk-prompt-values.json" (
      builtins.toJSON {
        btvirt = "${btvirt}/bin/btvirt";
        inherit passphrase port;
        qemuImg = "${nodes.laptop.virtualisation.qemu.package}/bin/qemu-img";
        image = "${diskoImages}/main.qcow2";
        real = "${toplevel}";
      }
    );
}
