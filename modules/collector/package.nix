# collector, built from this directory, its tests run as it builds. It
# takes its shape from the terminal kit (../terminal-kit), so the source is
# cut from the folder holding both.
{
  lib,
  pkg-config,
  rustPlatform,
  sqlite,
}:
rustPlatform.buildRustPackage rec {
  pname = "collector";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/collector";
  cargoLock.lockFile = ./Cargo.lock;
  # rusqlite links nixpkgs's SQLite, found through pkg-config, since
  # nixpkgs's cargo hook refuses the copy its bundled feature compiles.
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ sqlite ];
  meta = {
    description = "Keeps the traces an account's programs send it, to read a moment back";
    license = lib.licenses.mit;
    mainProgram = "collector";
    platforms = lib.platforms.linux;
  };
}
