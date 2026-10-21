# plymouth-waits, built from this directory's crate. Its tests run as it
# builds.
{
  dbus,
  lib,
  pkg-config,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "plymouth-waits";
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
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ dbus ];
  meta = {
    description = "Tells Plymouth what the boot or shutdown is waiting on";
    license = lib.licenses.mit;
    mainProgram = "plymouth-waits";
    platforms = lib.platforms.linux;
  };
}
