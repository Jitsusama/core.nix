# The guide, built from this directory, its tests run as it builds. Every
# program it runs is fixed here by its store path, so nothing on a PATH can
# take one's place, and `guide tools` proves each is there once it's built.
{
  dosfstools,
  lib,
  libfido2,
  rustPlatform,
  sbctl,
  # A machine gives its own, so bootctl matches the systemd-boot it installs.
  systemd,
  tpm2-tools,
  util-linux,
}:
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
  # The stick's tests are judged by what a person would check it with.
  nativeCheckInputs = [
    dosfstools
    util-linux
  ];
  env = {
    GUIDE_BOOTCTL = lib.getExe' systemd "bootctl";
    GUIDE_FIDO2_TOKEN = lib.getExe' libfido2 "fido2-token";
    GUIDE_LSBLK = lib.getExe' util-linux "lsblk";
    GUIDE_SBCTL = lib.getExe sbctl;
    GUIDE_TPM2_GETCAP = lib.getExe' tpm2-tools "tpm2_getcap";
  };
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
