# Whether a switch that brings in the MX Brio's sharing takes hold of a Brio
# already plugged in, rather than only of one plugged in after. What has to
# hold shows only with udev, logind's seat and systemd's devices there: the
# switch names the Brio for its relay and starts it, and takes back what the
# seat was handed before, so no program but the relay can open it. No VM has
# a Brio, so a loopback camera stands in, given the Brio's USB ids and its
# capture-only node by a rule of the test's own, as the kernel's uvcvideo and
# udev give them to the real one, so the seat's camera watch takes it as it
# took the Brio. A second loopback is any other camera, which keeps the seat's
# access through it all. The machine is a seat's, as a graphical one is, so
# a switch back to it gives the seat the Brio again with nobody replugging
# it, its give-back gone from the sharing as ../../../modules/mx-brio's
# checks pin. The tests are
# mx-brio-at-switch-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  card = "Stand-in Brio";
  other = "Another Camera";
  # Before mx-brio's own rule and the camera watch's, which read these, as
  # udev's usb_id and v4l_id set them for the Brio itself.
  likeTheBrio = pkgs.writeTextDir "lib/udev/rules.d/70-stand-in-brio.rules" ''
    SUBSYSTEM=="video4linux", ATTR{name}=="${card}", ENV{ID_USB_DRIVER}="uvcvideo", ENV{ID_VENDOR_ID}="046d", ENV{ID_MODEL_ID}="091d", ENV{ID_V4L_CAPABILITIES}=":capture:"
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
      imports = [
        self.nixosModules.mx-brio-seat
        self.nixosModules.usb-camera-holders
        self.nixosModules.v4l2loopback
      ];
      # The loopback's driver, which the module adds only for a camera it
      # names holders for, as mx-brio does once it's switched in.
      boot.extraModulePackages = [ config.jitsusama.v4l2loopback.package ];
      boot.extraModprobeConfig = "options v4l2loopback devices=0";
      services.udev.packages = [ likeTheBrio ];
      # The Brio, plugged in before the machine has the rules that share it,
      # and another camera beside it.
      systemd.services.stand-in-brio = {
        wantedBy = [ "multi-user.target" ];
        wants = [ "modprobe@v4l2loopback.service" ];
        after = [ "modprobe@v4l2loopback.service" ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = [
            "${loopback} add -n '${card}'"
            "${loopback} add -n '${other}'"
          ];
        };
      };
      # Someone at the seat, so logind hands its cameras to them, as it did
      # the Brio to Joel before the switch.
      users.users.someone.isNormalUser = true;
      services.getty.autologinUser = "someone";
      # getfacl reads the seat's access; v4l2loopback-ctl unplugs the stand-in
      # for the last test.
      environment.systemPackages = [
        pkgs.acl
        config.jitsusama.v4l2loopback.package.bin
      ];
      # A test machine's disk was never made to boot from, so GRUB can't be
      # installed on it; a switch with no boot loader still switches.
      boot.loader.grub.enable = false;
      # The machine as a switch leaves it, sharing the Brio.
      specialisation.shared.configuration.imports = [ self.nixosModules.mx-brio ];
    };

  tests = ./mx-brio-at-switch-vm-test.py;
  values = pkgs.writeText "mx-brio-at-switch-values.json" (builtins.toJSON { inherit card other; });
}
