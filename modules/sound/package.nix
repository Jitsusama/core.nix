# The sound daemon, built from this directory, its tests run as it builds.
# Its socket's tests ask it with a crate every daemon's tests share
# (../daemon-client), and its rows take the form every daemon's do
# (../ready-rows), and it names programs by their desktop entries and
# marks as the windows daemon does (../windows, ../programs, and
# ../daemon-serve, which the windows daemon answers through), so the
# source is cut from the modules folder holding them all, and built from
# this one. PipeWire's bindings are
# generated from its headers as they build, as dictation's are.
{
  lib,
  pipewire,
  pkg-config,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage rec {
  pname = "sound";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../chrome/session/Cargo.toml
      ../chrome/session/src
      ../daemon-client/Cargo.toml
      ../daemon-client/src
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../programs/Cargo.toml
      ../programs/src
      ../ready-rows/Cargo.toml
      ../ready-rows/src
      ../windows/Cargo.toml
      ../windows/src
    ];
  };
  sourceRoot = "${src.name}/sound";
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
  ];
  buildInputs = [ pipewire ];
  # A test starts the daemon as systemd does for sound.socket, through
  # systemd-socket-activate, handing it the socket it answers on.
  nativeCheckInputs = [ systemd ];
  meta = {
    description = "Holds the sound server's devices ready for the shell";
    license = lib.licenses.mit;
    mainProgram = "sound";
    platforms = lib.platforms.linux;
  };
}
