# The clipboard door a fenced pi command reaches, and the wl-copy and
# wl-paste it runs, built from this folder with their tests run
# (src/lib.rs says why). The shims are linked into a folder of their own,
# share/pi-fence/bin, which the fence puts first on a fenced command's
# PATH; nothing else should, since they reach only the door. They take
# their help from the terminal kit (../../terminal-kit), so the source is
# cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "pi-clipboard-door";
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
  sourceRoot = "${src.name}/pi/clipboard-door";
  cargoLock.lockFile = ./Cargo.lock;
  postInstall = ''
    mkdir -p $out/share/pi-fence/bin
    ln -s ../../../bin/pi-clipboard-shim $out/share/pi-fence/bin/wl-copy
    ln -s ../../../bin/pi-clipboard-shim $out/share/pi-fence/bin/wl-paste
  '';
  meta = {
    description = "The clipboard pi's fenced commands reach, through a door that never reads a password manager's secret";
    license = lib.licenses.mit;
    mainProgram = "pi-clipboard-door";
    platforms = lib.platforms.linux;
  };
}
