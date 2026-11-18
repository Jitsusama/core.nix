# The tool that draws the console's font from a face, built from this
# directory, its tests run as it builds. It takes its shape from the terminal
# kit, but from the kit at the commit pinned here rather than the working
# copy's ../../terminal-kit: the font it draws goes into the initrd, so every
# change to the kit would otherwise rebuild the initrd and need the boot
# tests. Cargo.toml keeps the path, and the pinned kit is put where it
# points, so cargo in the dev shell builds against today's kit and says when
# a bump would need this crate to change; its clippy check passes today's
# kit as terminalKit for the same reason. A bump (docs/commands.md) runs the
# VM tests that boot with the console's font: vm-boot-asks-in-the-theme,
# vm-boot-asks-on-every-screen, vm-boot-says-it-wakes-from-hibernation and
# the hibernation tests beside it.
{
  lib,
  fetchgit,
  rustPlatform,
  terminalKit ? fetchgit {
    url = "https://github.com/Jitsusama/core.nix";
    rev = "e10b9c29cac62608e02656076eebc8065f7a6d5d";
    rootDir = "modules/terminal-kit";
    hash = "sha256-vFJbmc1BUttpxrCIzl6TFcYXFIcWKZ7+vUvJe0sXSoo=";
  },
}:
rustPlatform.buildRustPackage rec {
  pname = "console-font";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  sourceRoot = "${src.name}/console/font";
  # The unpacked source is read-only above the source root.
  postUnpack = ''
    chmod u+w ${src.name}
    cp -r ${terminalKit} ${src.name}/terminal-kit
    chmod -R u+w ${src.name}/terminal-kit
  '';
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Draws a console font from an OpenType face";
    license = lib.licenses.mit;
    mainProgram = "console-font";
  };
}
