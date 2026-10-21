# The guide, built from this directory, its tests run as it builds. Every
# program it runs is fixed here by its store path, so nothing on a PATH can
# take one's place, and `guide tools` proves each is there once it's built.
{
  bash,
  btrfs-progs,
  coreutils,
  coreutils-full,
  cryptsetup,
  dosfstools,
  gawk,
  gnugrep,
  gnused,
  gptfdisk,
  jq,
  lib,
  libfido2,
  mtools,
  nix,
  nixos-enter,
  nixos-install,
  openssl,
  parted,
  rustPlatform,
  sbctl,
  sbsigntool,
  # A machine gives its own, so bootctl matches the systemd-boot it installs.
  systemd,
  systemdMinimal,
  tpm2-tools,
  util-linux,
}:
let
  # What a machine's own disko script finds on PATH when the stick lays its
  # disk out: the packages disko's _packages names for core's layout, with
  # bash, sed and awk its format-and-mount script adds.
  diskTools = [
    bash
    btrfs-progs
    coreutils-full
    cryptsetup
    dosfstools
    gawk
    gnugrep
    gnused
    gptfdisk
    jq
    parted
    systemdMinimal
    util-linux
  ];
in
rustPlatform.buildRustPackage {
  pname = "guide";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./fixtures
      ./src
      ./tests
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  # The stick's tests are judged by what a person would check it with, and
  # its signatures by sbsign's for the programs systemd builds for EFI.
  nativeCheckInputs = [
    dosfstools
    mtools
    openssl
    sbsigntool
    util-linux
  ];
  preCheck = ''
    export GUIDE_TEST_EFI=${systemd}/lib/systemd/boot/efi
  '';
  env = {
    GUIDE_BASH = lib.getExe bash;
    GUIDE_BOOTCTL = lib.getExe' systemd "bootctl";
    GUIDE_CRYPTSETUP = lib.getExe' cryptsetup "cryptsetup";
    GUIDE_FIDO2_ASSERT = lib.getExe' libfido2 "fido2-assert";
    GUIDE_FIDO2_TOKEN = lib.getExe' libfido2 "fido2-token";
    GUIDE_LSBLK = lib.getExe' util-linux "lsblk";
    GUIDE_NIX = lib.getExe nix;
    GUIDE_NIXOS_ENTER = lib.getExe nixos-enter;
    GUIDE_NIXOS_INSTALL = lib.getExe nixos-install;
    GUIDE_SBCTL = lib.getExe sbctl;
    GUIDE_SGDISK = lib.getExe' gptfdisk "sgdisk";
    GUIDE_SYSTEMD_CRYPTENROLL = lib.getExe' systemd "systemd-cryptenroll";
    GUIDE_SYSTEMCTL = lib.getExe' systemd "systemctl";
    GUIDE_TPM2_GETCAP = lib.getExe' tpm2-tools "tpm2_getcap";
    GUIDE_UDEVADM = lib.getExe' systemd "udevadm";
    GUIDE_UMOUNT = lib.getExe' util-linux "umount";
    GUIDE_WIPEFS = lib.getExe' util-linux "wipefs";

    GUIDE_DISK_TOOLS = lib.makeBinPath diskTools;
    GUIDE_DISK_PACKAGES = lib.concatMapStringsSep " " lib.getName diskTools;
    # nixos-install puts jq, nixos-enter and util-linux on its PATH and
    # takes nix-env and the rest from its caller's.
    GUIDE_INSTALL_TOOLS = lib.makeBinPath [
      coreutils
      nix
    ];
    GUIDE_TOKEN_PLUGINS = "${systemd}/lib/cryptsetup";
  }
  // (import ./theme.nix { inherit lib; }).env;
  doInstallCheck = true;
  installCheckPhase = ''
    runHook preInstallCheck
    $out/bin/guide tools
    runHook postInstallCheck
  '';
  meta = {
    description = "Walks a person through installing, repairing and sealing a machine";
    license = lib.licenses.mit;
    mainProgram = "guide";
    platforms = lib.platforms.linux;
  };
}
