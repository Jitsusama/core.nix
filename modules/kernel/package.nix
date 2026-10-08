# Joel's kernel: the stable 7.2 series as nixpkgs pins it, built with Clang
# and ThinLTO for one CPU, with the BORE scheduler and the settings in
# settings.nix. docs/kernel.md explains each choice, and how to move to a new
# series.
#
# Import this with nixpkgs rather than calling it with callPackage, so the
# kernel's override stays the one nixpkgs gives it: NixOS uses that to add
# the features a machine needs. Then give it the CPU, as Clang's -march names
# it, or null for any x86-64 machine, and the machine's AutoFDO profile, or
# null for none.
{
  lib,
  fetchpatch,
  kernelPatches,
  linux_7_2,
  llvmPackages_22,
  overrideCC,
  writeText,
  ...
}:
{ cpu, profile }:
let
  # Clang compiles and LLD links, which ThinLTO needs. LLVM 22 rather than
  # nixpkgs's default 21, whose LLD breaks objtool when compiled with GCC 16,
  # as nixpkgs compiles it: https://github.com/ClangBuiltLinux/linux/issues/2162
  llvm = llvmPackages_22;
  clangStdenv = overrideCC llvm.stdenv (llvm.stdenv.cc.override { inherit (llvm) bintools; });

  # Clang reads the CPU flags from a file because nixpkgs splits make flags on
  # spaces while it configures the kernel.
  cpuFlags = writeText "kernel-cpu-flags" "-march=${cpu} -mtune=${cpu}";
in
linux_7_2.override {
  stdenv = clangStdenv;

  # Compile for the machine's own CPU. The kernel adds these flags after the
  # generic -march and -mtune of its own Makefile, so they win. Then optimize
  # for what the profile recorded the kernel doing.
  extraMakeFlags = [
    "LLVM=1"
  ]
  ++ lib.optional (cpu != null) "KCFLAGS=@${cpuFlags}"
  ++ lib.optional (profile != null) "CLANG_AUTOFDO_PROFILE=${profile}";

  # Fail when nixpkgs's configuration or ours names an option this kernel
  # doesn't have, rather than dropping it silently.
  ignoreConfigErrors = false;

  # Where nixpkgs's common configuration sets one of these differently, ours
  # wins.
  structuredExtraConfig = lib.mapAttrs (_option: lib.mkForce) (
    import ./settings.nix { inherit lib; }
  );

  kernelPatches = [
    # The two patches nixpkgs applies to every kernel.
    kernelPatches.bridge_stp_helper
    kernelPatches.request_key_helper
    {
      # The BORE scheduler, from its author.
      name = "bore";
      patch = fetchpatch {
        name = "bore.patch";
        url = "https://raw.githubusercontent.com/firelzrd/bore-scheduler/1e2f57ad74e8b8b1d315c03b7abd8d74e40448b4/patches/stable/0001-linux7.2-rc1-bore-7.0.0.patch";
        hash = "sha256-6Qi12PX/YH9/PFfEbVexENJXo0uJeIulIYCwullqZqU=";
      };
    }
  ];
}
