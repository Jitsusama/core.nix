# shell-preview, for the dev shell, built from this directory's crate, with
# dbus-run-session and wtype on its path. Its tests run as it builds.
{
  dbus,
  lib,
  makeWrapper,
  rustPlatform,
  wtype,
}:
rustPlatform.buildRustPackage {
  pname = "shell-preview";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
    ];
  };
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
