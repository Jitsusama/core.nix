# bluetooth-again, built from this directory, its tests run as it builds,
# with coreutils' tee built in for sudo to run. It takes its shape from the
# terminal kit (../terminal-kit), so the source is cut from the folder
# holding both.
{
  lib,
  rustPlatform,
  coreutils,
}:
rustPlatform.buildRustPackage rec {
  pname = "bluetooth-again";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/bluetooth-again";
  cargoLock.lockFile = ./Cargo.lock;
  env.BLUETOOTH_AGAIN_TEE = lib.getExe' coreutils "tee";
  meta = {
    description = "Starts a Bluetooth controller that stopped answering again";
    license = lib.licenses.mit;
    mainProgram = "bluetooth-again";
  };
}
