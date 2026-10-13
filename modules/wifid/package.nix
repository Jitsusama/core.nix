# wifid, built from this directory's workspace without its sign-in window,
# which is a package of its own (sign-in/package.nix). Its tests run as it
# builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "wifid";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./daemon
      # Cargo reads every member of the workspace, though it builds one.
      ./sign-in/Cargo.toml
      ./sign-in/src
    ];
  };
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
