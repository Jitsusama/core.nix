# The checks of apple-macbook-pro-m4-pro-a3401, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  lib,
  nixpkgs,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  # The M4 Pro's kernel as a machine on it builds it, cross-compiled from
  # here, of which only the source and the patches are read.
  inherit
    ((nixpkgs.lib.nixosSystem {
      modules = [
        self.nixosModules.apple-macbook-pro-m4-pro-a3401
        { nixpkgs.buildPlatform = "x86_64-linux"; }
      ];
    }).config.boot.kernelPackages
    )
    kernel
    ;
  patches = lib.filter (patch: patch != null) (map (patch: patch.patch) kernel.kernelPatches);
in
onLinux {
  # The kernel's device tree for the J614s describes both external screens,
  # waiting for the boot loader to fill in their firmware's memory, with
  # their crossbars and HDMI's hot-plug pin, as its own ADT has them. Only
  # the device trees are compiled, with the C preprocessor and dtc, and the
  # rest of the kernel is never built, so it takes seconds and no aarch64
  # builder.
  m4-pro-kernel-describes-both-external-screens =
    pkgs.runCommand "m4-pro-kernel-describes-both-external-screens"
      {
        nativeBuildInputs = [
          pkgs.dtc
          pkgs.patchutils
        ];
      }
      ''
        ${lib.getExe pkgs.bash} ${./external-screens-are-described.sh} ${kernel.src} ${lib.escapeShellArgs patches}
        touch $out
      '';
}
