# The checks of microsoft-surface-laptop-4-1958, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  self,
  ...
}:
{
  # The Surface Laptop 4's keyboard is behind Surface's embedded controller,
  # so its disk can be unlocked from it only when the initrd has every driver
  # on the way there, and keyd gives that keyboard Colemak there too.
  the-surface-keyboard-types-at-the-disk-prompt =
    let
      surface = bareNixos [
        self.nixosModules.laptop
        self.nixosModules.microsoft-surface-laptop-4-1958
      ];
      inherit (surface.config) boot environment;
      path = [
        "8250_dw"
        "surface_aggregator"
        "surface_aggregator_registry"
        "surface_aggregator_hub"
        "surface_hid_core"
        "surface_hid"
      ];
      keyd = environment.etc."keyd/built-in.conf".text or "";
    in
    holds "the-surface-keyboard-types-at-the-disk-prompt"
      "The Surface Laptop 4's initrd lacks a driver on its keyboard's path, or keyd doesn't lay that keyboard out."
      (
        lib.all (module: lib.elem module boot.initrd.availableKernelModules) path
        && lib.hasInfix "045e:09ae" keyd
        && boot.initrd.systemd.contents ? "/etc/keyd/built-in.conf"
      );

  # The GPU's driver loads in the initrd, before the console is set up, and
  # the firmware's unreadable clock never becomes rtc0, and the CMOS clock
  # sets the time in the initrd, before systemd moves it forward.
  the-surface-boots-with-its-gpu-and-a-clock-that-reads =
    let
      surface = bareNixos [
        self.nixosModules.laptop
        self.nixosModules.microsoft-surface-laptop-4-1958
      ];
      inherit (surface.config) boot;
    in
    holds "the-surface-boots-with-its-gpu-and-a-clock-that-reads"
      "The Surface Laptop 4 loads amdgpu or rtc_cmos only after its initrd, or lets acpi_tad's unreadable clock take rtc0."
      (
        lib.elem "amdgpu" boot.initrd.kernelModules
        && lib.elem "rtc_cmos" boot.initrd.kernelModules
        && lib.elem "acpi_tad" boot.blacklistedKernelModules
      );
}
