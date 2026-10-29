# Runs Joel's kernel, from package.nix, compiled for the CPU the machine's
# hardware module names and optimized with the profile the machine recorded.
# Out-of-tree modules are built by the same compiler. Its panic screen wears
# the theme of the account the machine signs in, read as the console reads it
# (../greetd/account.nix); a machine without one keeps the kernel's.
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  inherit (import ../greetd/account.nix { inherit config lib options; }) theme;
  colours = lib.attrByPath [ "colours" ] null theme;
  panic = if colours == null then null else { inherit (colours) text background; };
in
{
  options.jitsusama.kernel.cpu = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "pantherlake";
    description = ''
      The CPU to compile the kernel for, as Clang's -march names it. The
      hardware module sets it; null compiles for any x86-64 machine.
    '';
  };

  options.jitsusama.kernel.profile = lib.mkOption {
    type = lib.types.nullOr lib.types.path;
    default = null;
    example = lib.literalExpression "./kernel.afdo";
    description = ''
      An AutoFDO profile of the kernel at work on this machine, which Clang
      optimizes the next build with. The machine sets it, since the profile is
      of what it runs; docs/kernel.md says how to record one.
    '';
  };

  options.jitsusama.kernel.packageOverlays = lib.mkOption {
    type = lib.types.listOf (lib.types.functionTo (lib.types.functionTo lib.types.attrs));
    default = [ ];
    example = lib.literalExpression ''
      [ (final: prev: { ipu7-drivers = prev.ipu7-drivers.overrideAttrs { patches = [ ./fix.patch ]; }; }) ]
    '';
    description = ''
      Overlays on the kernel's package set, in order, for a hardware module
      that has to change an out-of-tree module NixOS adds by itself, such as
      one it builds to match a patched kernel.
    '';
  };

  config.boot.kernelPackages =
    (pkgs.linuxPackagesFor (
      import ./package.nix pkgs {
        inherit (config.jitsusama.kernel) cpu profile;
        inherit panic;
      }
    )).extend
      (lib.composeManyExtensions config.jitsusama.kernel.packageOverlays);

  # Every device's DMA goes through the IOMMU, so Wi-Fi, Bluetooth or GPU
  # firmware can reach only the memory its driver gave it. Without this, an
  # Intel laptop whose firmware asks for DMA protection translates only its
  # Thunderbolt ports and lets every other device through untranslated. AMD's
  # IOMMU already translates everything and ignores the parameter.
  config.boot.kernelParams = [ "intel_iommu=on" ];
}
