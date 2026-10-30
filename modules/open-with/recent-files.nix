# recent-files alone, from this directory's crate, for what adds to the
# desktop's list of files opened lately without the portal or the shell
# beside it (src/bin/recent-files.rs says how it's used), with open-now,
# which says what a program has open now for the launcher to read
# (src/bin/open-now.rs). Their tests are package.nix's, which builds the
# whole crate.
{
  lib,
  rustPlatform,
  pkg-config,
  glib,
}:
rustPlatform.buildRustPackage (_finalAttrs: {
  pname = "recent-files";
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
  cargoBuildFlags = [
    "--bin"
    "recent-files"
    "--bin"
    "open-now"
  ];
  doCheck = false;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ glib ];

  meta = {
    description = "Adds to the desktop's list of files opened lately and reads it, and says what's open now";
    license = lib.licenses.mit;
    mainProgram = "recent-files";
    platforms = lib.platforms.linux;
  };
})
