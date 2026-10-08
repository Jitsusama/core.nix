# Runs Joel's kernel, from package.nix, compiled for the CPU the machine's
# hardware module names and optimized with the profile the machine recorded.
# Out-of-tree modules are built by the same compiler.
{
  config,
  lib,
  pkgs,
  ...
}:
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

  config.boot.kernelPackages = pkgs.linuxPackagesFor (
    import ./package.nix pkgs { inherit (config.jitsusama.kernel) cpu profile; }
  );
}
