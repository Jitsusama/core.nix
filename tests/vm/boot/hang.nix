# A machine set up so a hang leaves a record: hang-panic arms the watchdog
# and makes a hard lockup panic, and netconsole sends the kernel's log to a
# second machine as it's written. The listener runs darwin.nix's own socat
# command (listen.nix), which keeps only what comes from the test's network.
# QEMU's i6300ESB stands in for the laptop's iTCO, which the sleep hook stops
# and arms again, run by hand as systemd-sleep would. The machine's card has
# a name of its own, which udev gives it again when it comes back after
# being unplugged, as a dock's card is renamed from the kernel's eth0. The
# listener moves to another hardware address, and is followed there. The
# last test panics the kernel and reads the panic at the listener, then
# times the restart, which ends the test's VM, against the watchdog's. The
# tests are hang-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  inherit (pkgs) lib;
  qemu = import "${pkgs.path}/nixos/lib/qemu-common.nix" { inherit (pkgs) lib stdenv; };
  # What the tests need from Nix, their `values` fixture.
  log = "/var/log/netconsole.log";
  card = "dock0";
  values = pkgs.writeText "hang-values.json" (builtins.toJSON { inherit log card; });
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "hang";

  nodes.listener = {
    networking.firewall.allowedUDPPorts = [ 6666 ];
    # To send a line from outside the network named, which the listener drops.
    environment.systemPackages = [ pkgs.socat ];
    systemd.services.netconsole = {
      wantedBy = [ "multi-user.target" ];
      serviceConfig.ExecStart = lib.escapeShellArgs (
        import ../../../modules/netconsole/listen.nix {
          socat = lib.getExe pkgs.socat;
          port = 6666;
          from = "192.168.1.0/24";
          inherit log;
        }
      );
    };
  };

  nodes.machine =
    { config, ... }:
    {
      imports = [
        self.nixosModules.hang-panic
        self.nixosModules.netconsole
        # perf turns the NMI watchdog off, as on a workstation, for hang-panic to turn on again.
        self.nixosModules.perf
      ];
      virtualisation.qemu.options = [ "-device i6300esb" ];
      boot.kernelModules = [ "i6300esb" ];
      # The test's own rule names the card at boot, in the initrd; this one
      # names it when it comes back, as udev names the dock's card.
      virtualisation.interfaces.${card} = {
        vlan = 1;
        assignIP = true;
      };
      systemd.network.links."10-${card}" = {
        matchConfig.MACAddress = qemu.qemuNicMac 1 config.virtualisation.test.nodeNumber;
        linkConfig.Name = card;
      };
      jitsusama.netconsole = {
        listener = "listener";
        through = [ card ];
      };
    };

  tests = ./hang-vm-test.py;
  inherit values;
}
