# What a key says to the launcher, built from this directory, its tests run
# as it builds. It's linked statically, so running it reads one file and no
# library (src/main.rs says why). Its source reaches up to modules/ for the
# terminal kit, which answers its help, and builds from its own folder there.
{
  lib,
  rustPlatform,
  glibc,
  quickshell,
}:
rustPlatform.buildRustPackage rec {
  pname = "launcher-say";
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
  sourceRoot = "${src.name}/quickshell/launcher-say";
  cargoLock.lockFile = ./Cargo.lock;
  env = {
    # Static glibc reaches only the program's own link: the kit's crates
    # bring build scripts, linked dynamically, which a static libc on every
    # link's path breaks. Cargo keeps RUSTFLAGS off them, as nixpkgs names
    # the target.
    RUSTFLAGS = "-C target-feature=+crt-static -L native=${glibc.static}/lib";
    LAUNCHER_SAY_QS = lib.getExe quickshell;
  };
  meta = {
    description = "Says a chord to the launcher down its socket";
    license = lib.licenses.mit;
    mainProgram = "launcher-say";
  };
}
