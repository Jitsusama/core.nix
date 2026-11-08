# The M2 Air's kernel: Asahi's, at a tag, with Asahi's fairydust branch on
# top for a screen on the front-left USB-C port. The recipe is
# nixos-apple-silicon's (packages/linux-asahi, MIT), taken into core so core
# chooses the tag and the patches rather than waiting on it; its settings are
# Asahi's, each with the reason nixos-apple-silicon gives. Settings go in
# structuredExtraConfig rather than a patch's, so core's settings check
# reads them. A NixOS machine's boot.kernelPatches reach it through
# buildLinux's own kernelPatches, as with any nixpkgs kernel.
{
  lib,
  fetchFromGitHub,
  buildLinux,
  ...
}@args:
buildLinux (
  args
  // rec {
    pname = "linux-asahi";
    version = "7.2.15";
    modDirVersion = version;
    extraMeta.branch = "7.2";

    src = fetchFromGitHub {
      owner = "AsahiLinux";
      repo = "linux";
      tag = "asahi-7.2.15-2";
      hash = "sha256-q5kIQl0w/yn4dqM5rPqq2pFenxgi53XjCBa3MUlyQKI=";
    };

    # Asahi's drivers are written in Rust. A machine's own features, which
    # NixOS passes through an override, are kept beside it.
    features = {
      rust = true;
    }
    // (args.features or { });

    structuredExtraConfig = with lib.kernel; {
      # The GPU's driver needs 16K pages.
      ARM64_16K_PAGES = yes;
      ARM64_MEMORY_MODEL_CONTROL = yes;
      ARM64_ACTLR_STATE = yes;
      # Loaded too late, the watchdog restarts the machine.
      APPLE_WATCHDOG = yes;
      # It can't be a module, and is off by default.
      APPLE_M1_CPU_PMU = yes;
      # A module, so its options can be set in modprobe.d.
      HID_APPLE = module;
      APPLE_PMGR_MISC = yes;
      APPLE_PMGR_PWRSTATE = yes;
      # Without it Bluetooth audio stutters.
      BT_BRCMEXT = yes;
    };

    kernelPatches = [
      {
        # Janne Grunau's fairydust branch at 432e09dca5d8 (2026-11-05), the
        # 11 commits it holds beyond asahi-7.2.15-2, as GitHub's comparison
        # of the two gives them. It routes the second display coprocessor to
        # the front-left port's PHY, keeps that port's power domain on, and
        # passes the port controller's hot-plug signal to DRM: one external
        # screen, plugged in after the display driver has started. Asahi
        # ships it as-is, without support; kernel/README.md says what's
        # known to fail. It is cut for this tag and goes when Asahi's
        # kernel carries it, or is refreshed with the tag.
        name = "fairydust";
        patch = ./fairydust.patch;
      }
    ]
    ++ (args.kernelPatches or [ ]);
  }
)
