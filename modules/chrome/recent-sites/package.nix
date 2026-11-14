# chrome-recent-sites, built from this directory's crate with its tests run
# as it builds: the sites Chrome visited lately and its downloads, for its
# Recent in the launcher. It holds them to the rule of which pages are the
# web's that Chrome's open pages are held to (../session), and takes its
# shape from the terminal kit (../../terminal-kit), so the source is cut
# from the modules folder holding them all, and built from this one.
{
  lib,
  pkg-config,
  rustPlatform,
  sqlite,
}:
rustPlatform.buildRustPackage rec {
  pname = "chrome-recent-sites";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../session/Cargo.toml
      ../session/src
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/chrome/recent-sites";
  cargoLock.lockFile = ./Cargo.lock;
  # rusqlite links nixpkgs's SQLite, found through pkg-config, since
  # nixpkgs's cargo hook refuses the copy its bundled feature compiles.
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ sqlite ];
  meta = {
    description = "Prints the sites Chrome visited lately, or its downloads, newest first";
    license = lib.licenses.mit;
    mainProgram = "chrome-recent-sites";
    platforms = lib.platforms.linux;
  };
}
