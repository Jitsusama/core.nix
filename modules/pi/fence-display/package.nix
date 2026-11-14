# The display a fenced pi command reaches, a Wayland socket the compositor
# treats as a sandbox's, built from this folder with its tests run
# (src/main.rs says why). It takes its help from the terminal kit
# (../../terminal-kit), so the source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "pi-fence-display";
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
  sourceRoot = "${src.name}/pi/fence-display";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "A Wayland socket for pi's fenced commands that the compositor treats as a sandbox's";
    license = lib.licenses.mit;
    mainProgram = "pi-fence-display";
    platforms = lib.platforms.linux;
  };
}
