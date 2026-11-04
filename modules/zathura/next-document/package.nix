# zathura's ]b and [b, built from this directory's crate. Its tests run as
# it builds. It reaches zathura through glib's gdbus, named here.
{
  glib,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "zathura-next-document";
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
  env.GDBUS = lib.getExe' glib "gdbus";
  meta = {
    description = "Opens the next or the previous document in the folder of the one zathura shows";
    license = lib.licenses.mit;
    mainProgram = "zathura-next-document";
    platforms = lib.platforms.linux;
  };
}
