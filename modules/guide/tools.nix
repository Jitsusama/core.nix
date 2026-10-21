# Every program the guide runs, by the store path its build gives it, and
# the theme it draws in, as the variables `option_env!` reads when it's
# compiled (src/tool.rs, src/screen/mod.rs). The package builds with these,
# and so does core's development shell, so a guide built there finds its
# tools as the package's does.
{
  pkgs,
  # A machine gives its own, so bootctl matches the systemd-boot it installs.
  systemd ? pkgs.systemd,
}:
let
  inherit (pkgs) lib;
  # What a machine's own disko script finds on PATH when the stick lays its
  # disk out: the packages disko's _packages names for core's layout, with
  # bash, sed and awk its format-and-mount script adds.
  diskTools = [
    pkgs.bash
    pkgs.btrfs-progs
    pkgs.coreutils-full
    pkgs.cryptsetup
    pkgs.dosfstools
    pkgs.gawk
    pkgs.gnugrep
    pkgs.gnused
    pkgs.gptfdisk
    pkgs.jq
    pkgs.parted
    pkgs.systemdMinimal
    pkgs.util-linux
  ];
in
{
  GUIDE_BASH = lib.getExe pkgs.bash;
  GUIDE_BOOTCTL = lib.getExe' systemd "bootctl";
  GUIDE_CAT = lib.getExe' pkgs.coreutils "cat";
  GUIDE_CRYPTSETUP = lib.getExe' pkgs.cryptsetup "cryptsetup";
  GUIDE_FIDO2_ASSERT = lib.getExe' pkgs.libfido2 "fido2-assert";
  GUIDE_FIDO2_TOKEN = lib.getExe' pkgs.libfido2 "fido2-token";
  GUIDE_LSBLK = lib.getExe' pkgs.util-linux "lsblk";
  GUIDE_MOUNT = lib.getExe' pkgs.util-linux "mount";
  GUIDE_NIX = lib.getExe pkgs.nix;
  GUIDE_NIXOS_ENTER = lib.getExe pkgs.nixos-enter;
  GUIDE_NIXOS_INSTALL = lib.getExe pkgs.nixos-install;
  # systemd keeps systemd-pcrlock with its private programs, not in bin.
  GUIDE_PCRLOCK = "${systemd}/lib/systemd/systemd-pcrlock";
  GUIDE_RUN0 = lib.getExe' systemd "run0";
  GUIDE_SBCTL = lib.getExe pkgs.sbctl;
  GUIDE_SGDISK = lib.getExe' pkgs.gptfdisk "sgdisk";
  GUIDE_SYSTEMD_CRYPTENROLL = lib.getExe' systemd "systemd-cryptenroll";
  GUIDE_SYSTEMCTL = lib.getExe' systemd "systemctl";
  GUIDE_TPM2_GETCAP = lib.getExe' pkgs.tpm2-tools "tpm2_getcap";
  GUIDE_UDEVADM = lib.getExe' systemd "udevadm";
  GUIDE_UMOUNT = lib.getExe' pkgs.util-linux "umount";
  GUIDE_WIPEFS = lib.getExe' pkgs.util-linux "wipefs";

  GUIDE_DISK_TOOLS = lib.makeBinPath diskTools;
  GUIDE_DISK_PACKAGES = lib.concatMapStringsSep " " lib.getName diskTools;
  # nixos-install puts jq, nixos-enter and util-linux on its PATH and takes
  # nix-env and the rest from its caller's.
  GUIDE_INSTALL_TOOLS = lib.makeBinPath [
    pkgs.coreutils
    pkgs.nix
  ];
  GUIDE_TOKEN_PLUGINS = "${systemd}/lib/cryptsetup";
  # age finds its plugins on PATH, so the guide's own PATH is this alone.
  GUIDE_AGE_PLUGINS = lib.makeBinPath [ pkgs.age-plugin-fido2-hmac ];
}
// (import ./theme.nix { inherit lib; }).env
