# Joel's kernel: the 7.3 series, from kernel.org until nixpkgs has it, built
# with Clang and ThinLTO for one CPU, with the BORE scheduler and the settings
# in settings.nix. docs/kernel.md explains each choice, and how to move to a new
# series.
#
# Import this with nixpkgs rather than calling it with callPackage, so the
# kernel's override stays the one nixpkgs gives it: NixOS uses that to add
# the features a machine needs. Then give it the CPU, as Clang's -march names
# it, or null for any x86-64 machine, the machine's AutoFDO profile, or
# null for none, and the panic screen's colours, { text, background } as
# "#rrggbb", or null for the kernel's white on black.
{
  lib,
  fetchpatch,
  fetchurl,
  kernelPatches,
  linux_7_2,
  llvmPackages_22,
  overrideCC,
  writeText,
  ...
}:
{
  cpu,
  profile,
  panic ? null,
}:
let
  # Clang compiles and LLD links, which ThinLTO needs. LLVM 22 rather than
  # nixpkgs's default 21, whose LLD breaks objtool when compiled with GCC 16,
  # as nixpkgs compiles it: https://github.com/ClangBuiltLinux/linux/issues/2162
  llvm = llvmPackages_22;
  clangStdenv = overrideCC llvm.stdenv (llvm.stdenv.cc.override { inherit (llvm) bintools; });

  # Clang reads the CPU flags from a file because nixpkgs splits make flags on
  # spaces while it configures the kernel.
  cpuFlags = writeText "kernel-cpu-flags" "-march=${cpu} -mtune=${cpu}";

  # The newest stable release, until nixpkgs's pin reaches it. It applies
  # only while nixpkgs's is older, so it never holds the kernel back. The
  # pinned nixpkgs has no 7.3 package yet, so it's built from 7.2's, whose
  # build is the same. docs/kernel.md says why this release.
  stable = {
    version = "7.3";
    hash = "sha256-hzY8JPEOBjIes2J2VSEJTpRHH2v/i84taz5A858ASVM=";
  };
in
linux_7_2.override {
  stdenv = clangStdenv;

  argsOverride = lib.optionalAttrs (lib.versionOlder linux_7_2.version stable.version) {
    inherit (stable) version;
    # A first release, such as 7.3, puts its modules under 7.3.0.
    modDirVersion = lib.versions.pad 3 stable.version;
    src = fetchurl {
      url = "mirror://kernel/linux/kernel/v7.x/linux-${stable.version}.tar.xz";
      inherit (stable) hash;
    };
  };

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
    import ./settings.nix { inherit lib panic; }
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
        url = "https://raw.githubusercontent.com/firelzrd/bore-scheduler/1e2f57ad74e8b8b1d315c03b7abd8d74e40448b4/patches/stable/0001-linux7.3-rc2-bore-7.0.0.patch";
        hash = "sha256-InVLknDvy4C144bGV2MVw4PKOKXq2gJfcSc2vTnohqE=";
      };
    }
    {
      # Stops a resent interrupt from reaching handle_bad_irq() through a
      # nested parent with no handler, as an SDCA codec's does after resume.
      # Written here; docs/kernel.md has the cause.
      name = "irq-resend-nested";
      patch = ./irq-resend-nested.patch;
    }
    {
      # Lets the graphics driver keep its first eDP mode when the console
      # starts, rather than modeset the panel a second time, dark for a
      # second. Written here; docs/kernel.md has the cause.
      name = "edp-sdp-caps";
      patch = ./edp-sdp-caps.patch;
    }
    {
      # Keeps the name of the Wi-Fi network a card joined where networkd
      # reads it, when the access point names no country. Written here;
      # docs/kernel.md has the cause.
      name = "cfg80211-ssid-without-country";
      patch = ./cfg80211-ssid-without-country.patch;
    }
  ];
}
