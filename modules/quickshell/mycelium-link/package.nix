# What a mycelium: link opens in the launcher, built from this directory,
# its tests run as it builds. Its source reaches up to modules/ for the
# terminal kit, which answers its help and its refusals, and for the address
# cases its tests share, and builds from its own folder there.
{
  lib,
  rustPlatform,
  quickshell,
}:
rustPlatform.buildRustPackage rec {
  pname = "mycelium-link";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The address cases three copies of the rule are held to, and their
      # reader, which the tests take in (src/main.rs).
      ../../mycelium/site/tests/addresses.rs
      ../../mycelium/site/tests/addresses.txt
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/quickshell/mycelium-link";
  cargoLock.lockFile = ./Cargo.lock;
  env.MYCELIUM_LINK_QS = lib.getExe quickshell;
  meta = {
    description = "Opens what a mycelium: link names in the launcher";
    license = lib.licenses.mit;
    mainProgram = "mycelium-link";
  };
}
