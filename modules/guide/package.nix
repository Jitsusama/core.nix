# The guide, built from this directory, its tests run as it builds. Every
# program it runs is fixed by its store path (tools.nix), so nothing on a
# PATH can take one's place, and `guide tools` proves each is there once
# it's built.
{
  dosfstools,
  lib,
  mtools,
  openssl,
  pkgs,
  rustPlatform,
  sbsigntool,
  # A machine gives its own, so bootctl matches the systemd-boot it installs.
  systemd,
  # A machine showing a splash gives its Plymouth (tools.nix).
  bootSplash ? null,
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
  env = import ./tools.nix { inherit bootSplash pkgs systemd; };
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
