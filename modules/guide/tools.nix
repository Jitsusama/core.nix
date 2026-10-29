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
  GUIDE_BTRFS = lib.getExe' pkgs.btrfs-progs "btrfs";
  GUIDE_CAT = lib.getExe' pkgs.coreutils "cat";
  GUIDE_CHOWN = lib.getExe' pkgs.coreutils "chown";
  GUIDE_CRYPTSETUP = lib.getExe' pkgs.cryptsetup "cryptsetup";
  GUIDE_CURL = lib.getExe pkgs.curl;
  GUIDE_ENV = lib.getExe' pkgs.coreutils "env";
  GUIDE_DF = lib.getExe' pkgs.coreutils "df";
  GUIDE_DMSETUP = lib.getExe' pkgs.lvm2 "dmsetup";
  GUIDE_DU = lib.getExe' pkgs.coreutils "du";
  GUIDE_FIDO2_ASSERT = lib.getExe' pkgs.libfido2 "fido2-assert";
  GUIDE_FIDO2_TOKEN = lib.getExe' pkgs.libfido2 "fido2-token";
  GUIDE_GH = lib.getExe pkgs.gh;
  GUIDE_LOSETUP = lib.getExe' pkgs.util-linux "losetup";
  GUIDE_LSBLK = lib.getExe' pkgs.util-linux "lsblk";
  GUIDE_MKDIR = lib.getExe' pkgs.coreutils "mkdir";
  GUIDE_MV = lib.getExe' pkgs.coreutils "mv";
  GUIDE_MOUNT = lib.getExe' pkgs.util-linux "mount";
  GUIDE_NIX = lib.getExe pkgs.nix;
  GUIDE_NIXOS_ENTER = lib.getExe pkgs.nixos-enter;
  GUIDE_NIXOS_INSTALL = lib.getExe pkgs.nixos-install;
  # systemd keeps systemd-pcrlock with its private programs, not in bin.
  GUIDE_PCRLOCK = "${systemd}/lib/systemd/systemd-pcrlock";
  GUIDE_RUN0 = lib.getExe' systemd "run0";
  GUIDE_RSYNC = lib.getExe pkgs.rsync;
  GUIDE_RUNUSER = lib.getExe' pkgs.util-linux "runuser";
  GUIDE_SBCTL = lib.getExe pkgs.sbctl;
  GUIDE_SGDISK = lib.getExe' pkgs.gptfdisk "sgdisk";
  # Built for the guide alone, so nothing else on a machine rebuilds: its
  # spares step reads a YubiKey holding no resident keys as having none to
  # download, where OpenSSH 10.5p1 fails (openssh-empty-key.patch says why).
  GUIDE_SSH_KEYGEN = lib.getExe' (pkgs.openssh.overrideAttrs (old: {
    patches = old.patches or [ ] ++ [ ./openssh-empty-key.patch ];
  })) "ssh-keygen";
  GUIDE_SSH_TPM_KEYGEN = lib.getExe' pkgs.ssh-tpm-agent "ssh-tpm-keygen";
  GUIDE_SYSTEMD_CRYPTENROLL = lib.getExe' systemd "systemd-cryptenroll";
  GUIDE_SYSTEMCTL = lib.getExe' systemd "systemctl";
  GUIDE_TPM2_CREATEPRIMARY = lib.getExe' pkgs.tpm2-tools "tpm2_createprimary";
  GUIDE_TPM2_GETCAP = lib.getExe' pkgs.tpm2-tools "tpm2_getcap";
  GUIDE_TPM2_LOAD = lib.getExe' pkgs.tpm2-tools "tpm2_load";
  GUIDE_TPM2_NVREADPUBLIC = lib.getExe' pkgs.tpm2-tools "tpm2_nvreadpublic";
  GUIDE_TPM2_NVUNDEFINE = lib.getExe' pkgs.tpm2-tools "tpm2_nvundefine";
  GUIDE_UDEVADM = lib.getExe' systemd "udevadm";
  GUIDE_UMOUNT = lib.getExe' pkgs.util-linux "umount";
  GUIDE_VERITYSETUP = lib.getExe' pkgs.cryptsetup "veritysetup";
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
