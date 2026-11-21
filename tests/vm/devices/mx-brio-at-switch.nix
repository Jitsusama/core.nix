# Whether a switch that brings in the MX Brio's sharing takes hold of a Brio
# already plugged in, rather than only of one plugged in after. What has to
# hold shows only with udev, logind's seat and systemd's devices there: the
# switch names the Brio for its relay and starts it, and takes back what the
# seat was handed before, so no program but the relay can open it. No VM has
# a Brio, so a loopback camera stands in, given the Brio's USB ids by a rule
# of the test's own, as the kernel's uvcvideo and udev's usb_id give them to
# the real one. The tests are mx-brio-at-switch-vm-test.py, which pytest runs
# inside the driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  card = "Stand-in Brio";
  # Before mx-brio's own rule, which reads these, as udev's usb_id sets them
  # for the Brio itself.
  likeTheBrio = pkgs.writeTextDir "lib/udev/rules.d/70-stand-in-brio.rules" ''
    SUBSYSTEM=="video4linux", ATTR{name}=="${card}", ENV{ID_USB_DRIVER}="uvcvideo", ENV{ID_VENDOR_ID}="046d", ENV{ID_MODEL_ID}="091d"
  '';
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "mx-brio-at-switch";

  nodes.machine =
    { config, ... }:
    let
      loopback = "${config.jitsusama.v4l2loopback.package.bin}/bin/v4l2loopback-ctl";
    in
    {
      imports = [ self.nixosModules.v4l2loopback ];
      # The loopback's driver, which the module adds only for a camera it
      # names holders for, as mx-brio does once it's switched in.
      boot.extraModulePackages = [ config.jitsusama.v4l2loopback.package ];
      boot.extraModprobeConfig = "options v4l2loopback devices=0";
      services.udev.packages = [ likeTheBrio ];
      # The Brio, plugged in before the machine has the rules that share it.
      systemd.services.stand-in-brio = {
        wantedBy = [ "multi-user.target" ];
        wants = [ "modprobe@v4l2loopback.service" ];
        after = [ "modprobe@v4l2loopback.service" ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = "${loopback} add -n '${card}'";
        };
      };
      # Someone at the seat, so logind hands its cameras to them, as it did
      # the Brio to Joel before the switch.
      users.users.someone.isNormalUser = true;
      services.getty.autologinUser = "someone";
      environment.systemPackages = [ pkgs.acl ];
      # A test machine's disk was never made to boot from, so GRUB can't be
      # installed on it; a switch with no boot loader still switches.
      boot.loader.grub.enable = false;
      # The machine as a switch leaves it, sharing the Brio.
      specialisation.shared.configuration.imports = [ self.nixosModules.mx-brio ];
    };

  tests = ./mx-brio-at-switch-vm-test.py;
  values = pkgs.writeText "mx-brio-at-switch-values.json" (builtins.toJSON { inherit card; });
}
