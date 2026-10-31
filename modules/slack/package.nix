# slack-people and slack-unreads, built from this directory, their tests run
# as they build.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "slack-people";
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
    description = "Says who Slack knows by an address, and what in Slack wants Joel, only reading";
    license = lib.licenses.mit;
    mainProgram = "slack-people";
    platforms = lib.platforms.linux;
  };
}
