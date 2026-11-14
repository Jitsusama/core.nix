# Joel's timers, kept and rung for the launcher's Timers, built from this
# directory, its tests run as it builds. It takes its shape from the
# terminal kit (../../terminal-kit), so the source is cut from the folder
# holding both.
{
  dbus,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "timers";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/quickshell/timers";
  cargoLock.lockFile = ./Cargo.lock;
  # A test runs a bus of its own, with a stand-in notifications server on
  # it, for the daemon to ring on.
  nativeCheckInputs = [ dbus ];
  meta = {
    description = "Keeps Joel's timers and rings each as a notice, for the launcher's Timers";
    license = lib.licenses.mit;
    mainProgram = "timers";
    platforms = lib.platforms.linux;
  };
}
