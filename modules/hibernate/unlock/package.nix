# hibernate-unlock, built from this directory, its tests run as it builds.
# It takes its shape from the terminal kit, but from the kit at the commit
# pinned here rather than the working copy's ../../terminal-kit: the initrd
# holds what this builds, so every change to the kit would otherwise rebuild
# the initrd and need the boot tests. Cargo.toml keeps the path, and the
# pinned kit is put where it points, so cargo in the dev shell builds
# against today's kit and says when a bump would need this crate to change.
# A bump runs vm-hibernation-comes-back and
# vm-boot-says-it-wakes-from-hibernation.
{
  lib,
  fetchgit,
  rustPlatform,
}:
let
  kit = fetchgit {
    url = "https://github.com/Jitsusama/core.nix";
    rev = "b9374ddb10cc664e1a1c4f159cd13df6c121ed57";
    rootDir = "modules/terminal-kit";
    hash = "sha256-6PoA31idF7JayVeQs7RO69DQl/CzaV98+pK0DLZBsxQ=";
  };
in
rustPlatform.buildRustPackage rec {
  pname = "hibernate-unlock";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  sourceRoot = "${src.name}/hibernate/unlock";
  # The unpacked source is read-only above the source root.
  postUnpack = ''
    chmod u+w ${src.name}
    cp -r ${kit} ${src.name}/terminal-kit
    chmod -R u+w ${src.name}/terminal-kit
  '';
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Lets the lock go after a resume from hibernation, and rescues one that rolled back";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "hibernate-unlock";
  };
}
