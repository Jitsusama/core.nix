# wifid, built from this directory's workspace without its sign-in window,
# which is a package of its own (sign-in/package.nix). Its tests run as it
# builds, and given the launcher's words, they read Wi-Fi's from them too.
# Its rows take the form every daemon's do (../ready-rows), so the source is
# cut from the modules folder holding both, and built from this one.
{
  lib,
  rustPlatform,
  words ? null,
}:
rustPlatform.buildRustPackage rec {
  pname = "wifid";
  env = lib.optionalAttrs (words != null) { WIFID_TEST_WORDS = "${words}"; };
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./daemon
      # Cargo reads every member of the workspace, though it builds one.
      ./sign-in/Cargo.toml
      ./sign-in/src
      ../ready-rows/Cargo.toml
      ../ready-rows/src
    ];
  };
  sourceRoot = "${src.name}/wifid";
  cargoLock.lockFile = ./Cargo.lock;
  cargoBuildFlags = [
    "--package"
    "wifid"
  ];
  cargoTestFlags = [
    "--package"
    "wifid"
  ];
  meta = {
    description = "Joins Wi-Fi networks and finds the ones that want a sign-in";
    license = lib.licenses.mit;
    mainProgram = "wifid";
    platforms = lib.platforms.linux;
  };
}
