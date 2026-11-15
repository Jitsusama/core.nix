# The tool that draws the console's font from a face, built from this
# directory, its tests run as it builds. It takes its shape from the terminal
# kit, but from the kit at the commit pinned here rather than the working
# copy's ../../terminal-kit: the font it draws goes into the initrd, so every
# change to the kit would otherwise rebuild the initrd and need the boot
# tests. Cargo.toml keeps the path, and the pinned kit is put where it
# points, so cargo in the dev shell builds against today's kit and says when
# a bump would need this crate to change. A bump runs the VM tests that
# boot with the console's font, vm-boot-says-it-wakes-from-hibernation
# among them.
{
  lib,
  fetchgit,
  rustPlatform,
}:
let
  kit = fetchgit {
    url = "https://github.com/Jitsusama/core.nix";
    rev = "5d502f291f10c1968e9e7d50c340e9ffa32d4b48";
    rootDir = "modules/terminal-kit";
    hash = "sha256-jYS3//iS1/r2wRbVhaZBX1ieI7xK96G1uNCzIV3lgyg=";
  };
in
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
    cp -r ${kit} ${src.name}/terminal-kit
    chmod -R u+w ${src.name}/terminal-kit
  '';
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Draws a console font from an OpenType face";
    license = lib.licenses.mit;
    mainProgram = "console-font";
  };
}
