# zathura's commands through the launcher's command line, built from this
# directory's crate. Its tests run as it builds. It reaches zathura through
# glib's gdbus, named here.
{
  glib,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "zathura-command";
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
    description = "Carries the launcher's command line to zathura, and opens it from its colon";
    license = lib.licenses.mit;
    mainProgram = "zathura-command";
    platforms = lib.platforms.linux;
  };
}
