# plymouth-waits, built from this directory's crate. Its tests run as it
# builds. It takes its shape from the terminal kit (../../terminal-kit), so
# the source is cut from the folder holding both.
{
  dbus,
  lib,
  pkg-config,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "plymouth-waits";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/plymouth/waits";
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
