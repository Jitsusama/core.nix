# a-call-is-on, built from this directory's crate, with pw-dump on its path
# on Linux; a Mac's asks CoreAudio, from the SDK its stdenv builds with.
# Its tests run as it builds, against a stand-in pw-dump on Linux.
{
  lib,
  stdenv,
  makeWrapper,
  pipewire,
  runtimeShell,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "a-call-is-on";
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
  # The stand-in pw-dump runs under the build's own shell, since the
  # sandbox has no /bin/sh.
  env.STAND_IN_SHELL = runtimeShell;
  postInstall = lib.optionalString stdenv.hostPlatform.isLinux ''
    wrapProgram $out/bin/a-call-is-on --prefix PATH : ${lib.makeBinPath [ pipewire ]}
  '';
  meta = {
    description = "Says whether a call is on";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
    mainProgram = "a-call-is-on";
  };
}
