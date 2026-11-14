# pain-points, built from this directory, its tests run as it builds.
{
  lib,
  pkg-config,
  rustPlatform,
  sqlite,
}:
rustPlatform.buildRustPackage {
  pname = "pain-points";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  # rusqlite links nixpkgs's SQLite, found through pkg-config, since
  # nixpkgs's cargo hook refuses the copy its bundled feature compiles.
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ sqlite ];
  meta = {
    description = "Names the interactions with the launcher that slowed Joel most today";
    license = lib.licenses.mit;
    mainProgram = "pain-points";
    platforms = lib.platforms.linux;
  };
}
