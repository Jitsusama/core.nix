# The 14-inch M4 Pro's kernel: Aurora Silicon's, at a commit, since Asahi's
# own tree has no disk, USB or GPU for this chip yet. The commit is the one
# Aurora's NixOS flake (nixos-aurora, MIT) pins, so its source tarball and
# hash are the ones that flake already checks. It is built like the M2 Air's
# kernel (../../apple-macbook-air-m2-a2681/kernel/package.nix): the ARM64
# defconfig, nixpkgs' common settings, and then Aurora's shared Apple
# fragment, asahi.config, taken in whole as structured settings so core's
# settings check holds every one of them. A NixOS machine's
# boot.kernelPatches reach it through buildLinux's own kernelPatches.
{
  lib,
  fetchurl,
  buildLinux,
  ...
}@args:
let
  inherit (lib.kernel)
    yes
    no
    module
    freeform
    ;

  # A line of a kernel configuration fragment as a structured setting, or
  # nothing for a comment.
  setting =
    line:
    let
      set = builtins.match "CONFIG_([A-Za-z0-9_]+)=(.*)" line;
      unset = builtins.match "# CONFIG_([A-Za-z0-9_]+) is not set" line;
      value =
        answer:
        if answer == "y" then
          yes
        else if answer == "m" then
          module
        else if answer == "n" then
          no
        else
          freeform (lib.removePrefix "\"" (lib.removeSuffix "\"" answer));
    in
    if set != null then
      [ (lib.nameValuePair (lib.elemAt set 0) (value (lib.elemAt set 1))) ]
    else if unset != null then
      [ (lib.nameValuePair (lib.elemAt unset 0) no) ]
    else
      [ ];

  # Aurora's arch/arm64/configs/asahi.config at the commit below, as it says
  # to use it: over the ARM64 defconfig, then resolved. Its settings outrank
  # nixpkgs' common ones, which are written for any machine, and give way to
  # a machine's own lib.mkForce.
  aurora = lib.mapAttrs (_option: lib.mkOverride 90) (
    lib.listToAttrs (lib.concatMap setting (lib.splitString "\n" (builtins.readFile ./asahi.config)))
  );
in
buildLinux (
  args
  // rec {
    pname = "linux-aurora";
    version = "7.3.0";
    modDirVersion = version;
    extraMeta.branch = "7.3";

    # integration/aurora at d4d267839caa (2026-11-08), as nixos-aurora's
    # sources.json pins it. It carries the J614s's board file and its
    # 16-core GPU, and the external display chain both 14- and 16-inch
    # M4 Pros share (README.md).
    src = fetchurl {
      name = "linux-aurora-d4d267839caa.tar.gz";
      url = "https://codeload.github.com/aurora-silicon/linux/tar.gz/d4d267839caa0aa9cbcd98a792351a52bd856b84";
      hash = "sha256-P9JNVztZcQL3SeWtAQw9u2hNmmla8hvTHJno+nxwX+s=";
    };

    # Apple's drivers here are written in Rust, as on the M2. A machine's
    # own features, which NixOS passes through an override, are kept beside
    # it.
    features = {
      rust = true;
    }
    // (args.features or { });

    structuredExtraConfig = aurora // {
      # Aurora's fragment leaves it to the base, and nothing here can
      # resume: as on the M2, the boot chain keeps no EFI variables to say
      # where the image went, and core's Apple laptops leave the laptop
      # role's hibernation out.
      HIBERNATION = no;
    };

    kernelPatches = args.kernelPatches or [ ];
  }
)
