# The 14-inch M4 Pro runs core's build of Aurora Silicon's kernel
# (package.nix). What the boot chain needs beside it, its initrd and its
# kernel's command line, comes with the boot chain itself.
{ lib, pkgs, ... }:
{
  boot.kernelPackages = pkgs.linuxPackagesFor (pkgs.callPackage ./package.nix { });

  # schedutil, for the kernel's energy model, which knows the efficiency
  # cores from the performance ones.
  powerManagement.cpuFreqGovernor = lib.mkOverride 800 "schedutil";
}
