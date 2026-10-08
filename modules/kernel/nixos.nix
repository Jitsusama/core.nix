# Runs Joel's kernel, from package.nix, compiled for the CPU the machine's
# hardware module names. Out-of-tree modules are built by the same compiler.
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

  config.boot.kernelPackages = pkgs.linuxPackagesFor (
    import ./package.nix pkgs { inherit (config.jitsusama.kernel) cpu; }
  );
}
