# wifid-calls, built from this directory, with pw-dump on its path. Its
# tests run as it builds. It takes what counts as a call from
# modules/calls' crate, which takes its shape from the terminal kit, so
# the source is cut from the modules folder holding all three.
{
  lib,
  makeWrapper,
  pipewire,
  rustPlatform,
}:
let
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../calls/Cargo.toml
      ../../calls/src
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
in
rustPlatform.buildRustPackage {
  pname = "wifid-calls";
  version = "0.1.0";
  inherit src;
  sourceRoot = "${src.name}/wifid/calls";
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];
  postInstall = ''
    wrapProgram $out/bin/wifid-calls --prefix PATH : ${lib.makeBinPath [ pipewire ]}
  '';
  meta = {
    description = "Tells wifid when a call is on";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "wifid-calls";
  };
}
