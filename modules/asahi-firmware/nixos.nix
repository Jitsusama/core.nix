# The firmware an Apple silicon Mac's Wi-Fi, Bluetooth and camera need,
# which Apple doesn't let anyone pass on, so Asahi's installer copies it out
# of macOS onto the EFI system partition. The initrd unpacks it from there
# at each start (unpack.sh) and copies it into the running system, rather
# than reading it while evaluating, which a machine built anywhere but on
# itself, or by a pure evaluation, can't do. It needs systemd in the
# initrd, which is the only initrd nixpkgs builds now. It follows
# nixos-apple-silicon's peripheral-firmware module (MIT, under the notice
# in LICENSE.nixos-apple-silicon), the path that reads it at boot.
{ lib, pkgs, ... }:
{
  # To mount the EFI system partition.
  boot.initrd.availableKernelModules = [
    "vfat"
    "nls_cp437"
    "nls_iso8859-1"
  ];

  boot.initrd.systemd = {
    extraBin.cpio = lib.getExe pkgs.cpio;

    mounts = [
      {
        what = "vendorfw";
        where = "/sysroot/lib/firmware/vendor";
        type = "tmpfs";
        options = "mode=0755";
        wantedBy = [ "initrd-fs.target" ];
      }
    ];

    services.asahi-peripheral-firmware = {
      description = "Extract Asahi peripheral firmware from the EFI system partition";
      before = [
        "systemd-udevd.service"
        "systemd-udev-trigger.service"
        "sysinit.target"
        "shutdown.target"
      ];
      conflicts = [ "shutdown.target" ];
      wantedBy = [ "sysinit.target" ];
      unitConfig.DefaultDependencies = false;
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
      };
      script = builtins.readFile ./unpack.sh;
    };

    # From the initrd into the running system, as asahi-scripts'
    # install-asahi-firmware.sh does.
    services.asahi-peripheral-firmware-install = {
      description = "Install Asahi peripheral firmware to the root filesystem";
      wantedBy = [ "initrd.target" ];
      before = [ "initrd.target" ];
      unitConfig = {
        ConditionPathExists = "/vendorfw";
        RequiresMountsFor = "/sysroot/lib/firmware/vendor";
      };
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = "/bin/cp -a /vendorfw/. /sysroot/lib/firmware/vendor";
      };
    };
  };
}
