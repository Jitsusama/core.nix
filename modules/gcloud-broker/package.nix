# gcloud-broker, built from this directory, its tests run as it builds. It
# runs the gcloud on the caller's PATH, the one the person's login was made
# with, rather than one of its own, which could read that login's files in
# another version's shape.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "gcloud-broker";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Mints a service account's hour-long Google Cloud tokens with the person's login";
    license = lib.licenses.mit;
    mainProgram = "gcloud-broker";
  };
}
