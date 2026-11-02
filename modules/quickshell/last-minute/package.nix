# The last minute of the screen and its sound for the shell's recorder, built
# from this directory, its tests run as it builds. wf-recorder records and
# ffmpeg copies a saved minute into its file.
{
  lib,
  rustPlatform,
  makeWrapper,
  wf-recorder,
  ffmpeg-headless,
}:
rustPlatform.buildRustPackage {
  pname = "last-minute";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];
  postInstall = ''
    wrapProgram $out/bin/last-minute \
      --prefix PATH : ${
        lib.makeBinPath [
          wf-recorder
          ffmpeg-headless
        ]
      }
  '';
  meta = {
    description = "Keeps the last minute of the screen in memory, saved when asked";
    license = lib.licenses.mit;
    mainProgram = "last-minute";
    platforms = lib.platforms.linux;
  };
}
