# net, built from this directory, its tests run as it builds. Its command
# line takes the shape every command's does (../terminal-kit), and its
# daemon serves as every daemon does (../daemon-serve), so the source is
# cut from the modules folder holding it, and built from this one.
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "net";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The record every view's tests draw from, and what it's collected
      # from: a real modem's pages and eero's golden record.
      ./tests/network.json
      ./contract
      ../eero/home-network.json
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/net";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Reads the whole home network, every device and how they link";
    license = lib.licenses.mit;
    mainProgram = "net";
  };
}
