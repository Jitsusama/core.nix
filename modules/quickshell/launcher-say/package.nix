# What a key says to the launcher, built from this directory, its tests run
# as it builds. It's linked statically, so running it reads one file and no
# library (src/main.rs says why).
{
  lib,
  rustPlatform,
  glibc,
  quickshell,
}:
rustPlatform.buildRustPackage {
  pname = "launcher-say";
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
  buildInputs = [ glibc.static ];
  env = {
    RUSTFLAGS = "-C target-feature=+crt-static";
    LAUNCHER_SAY_QS = lib.getExe quickshell;
  };
  meta = {
    description = "Says a chord to the launcher down its socket";
    license = lib.licenses.mit;
    mainProgram = "launcher-say";
  };
}
