# What a failed boot's console says, built from this directory, its tests
# run as it builds. It takes its shape from the terminal kit, but from the
# kit at the commit pinned here rather than the working copy's
# ../../terminal-kit: the initrd carries it, so every change to the kit
# would otherwise rebuild the initrd and need the boot tests. Cargo.toml
# keeps the path, and the pinned kit is put where it points, so cargo in the
# dev shell builds against today's kit and says when a bump would need this
# crate to change; its clippy check passes today's kit as terminalKit for
# the same reason. A bump (docs/commands.md) runs the VM tests that read a
# failed boot's console: vm-boot-asks-in-the-theme's emergency and initrd
# subtests (tests/vm/boot/plymouth.nix).
{
  lib,
  fetchgit,
  rustPlatform,
  terminalKit ? fetchgit {
    url = "https://github.com/Jitsusama/core.nix";
    rev = "3840777a489b0aae5a59f407aa1f91731d455fc9";
    rootDir = "modules/terminal-kit";
    hash = "sha256-aSKWzdIyfw2kZkTqIpMZAiEW2GvO807VHokspsCu2Zo=";
  },
}:
rustPlatform.buildRustPackage rec {
  pname = "boot-stopped";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  sourceRoot = "${src.name}/console/stopped";
  # The unpacked source is read-only above the source root.
  postUnpack = ''
    chmod u+w ${src.name}
    cp -r ${terminalKit} ${src.name}/terminal-kit
    chmod -R u+w ${src.name}/terminal-kit
  '';
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says on the console what stopped the boot";
    license = lib.licenses.mit;
    mainProgram = "boot-stopped";
  };
}
