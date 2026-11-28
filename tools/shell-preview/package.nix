# shell-preview, for the dev shell, built from this directory's crate, with
# dbus-run-session and wtype on its path. Its tests run as it builds. It
# takes its shape from the terminal kit (../../modules/terminal-kit), and its
# tests time their steps by the test pace (../../modules/test-pace), so the
# source is cut from the folder holding all three.
{
  dbus,
  lib,
  makeWrapper,
  rustPlatform,
  wtype,
}:
rustPlatform.buildRustPackage rec {
  pname = "shell-preview";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../modules/terminal-kit/Cargo.toml
      ../../modules/terminal-kit/src
      ../../modules/test-pace/Cargo.toml
      ../../modules/test-pace/src
    ];
  };
  sourceRoot = "${src.name}/tools/shell-preview";
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];
  postInstall = ''
    wrapProgram $out/bin/shell-preview --prefix PATH : ${
      lib.makeBinPath [
        dbus
        wtype
      ]
    }
  '';
  meta = {
    description = "Shows the desktop's shell from the working copy, in a niri window of its own";
    license = lib.licenses.mit;
    mainProgram = "shell-preview";
    platforms = lib.platforms.linux;
  };
}
