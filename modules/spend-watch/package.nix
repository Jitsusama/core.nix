# spend-watch, built from this directory, its tests run as it builds. Its
# rows take the form every daemon's do (../ready-rows) and its command line
# the terminal kit's (../terminal-kit), so the source is cut from the
# modules folder holding all three, and built from this one.
{
  lib,
  pkg-config,
  rustPlatform,
  sqlite,
}:
rustPlatform.buildRustPackage rec {
  pname = "spend-watch";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The feed contract's examples, which its tests read.
      ./contract
      ../ready-rows/Cargo.toml
      ../ready-rows/src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/spend-watch";
  cargoLock.lockFile = ./Cargo.lock;
  # rusqlite links nixpkgs's SQLite, found through pkg-config, since
  # nixpkgs's cargo hook refuses the copy its bundled feature compiles.
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ sqlite ];
  meta = {
    description = "Keeps an account told what its AI use costs";
    license = lib.licenses.mit;
    mainProgram = "spend-watch";
    platforms = lib.platforms.linux;
  };
}
