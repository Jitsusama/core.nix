# file-preview alone, from this directory's crate, for the launcher's
# chooser of files to ask what a file shows of itself (src/bin/
# file-preview.rs says how), with poppler's and FFmpeg's programs it draws
# a PDF's first page and a video's frame with on its path. Its tests are
# package.nix's, which builds the whole crate. The source is cut from the
# folder holding the terminal kit too, as package.nix's is.
{
  lib,
  rustPlatform,
  makeWrapper,
  pkg-config,
  glib,
  poppler-utils,
  ffmpeg-headless,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "file-preview";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${finalAttrs.src.name}/open-with";
  cargoLock.lockFile = ./Cargo.lock;
  cargoBuildFlags = [
    "--bin"
    "file-preview"
  ];
  doCheck = false;

  nativeBuildInputs = [
    makeWrapper
    pkg-config
  ];
  # The crate's library links GLib, though this program doesn't use it.
  buildInputs = [ glib ];

  postInstall = ''
    wrapProgram $out/bin/file-preview \
      --prefix PATH : ${
        lib.makeBinPath [
          poppler-utils
          ffmpeg-headless
        ]
      }
  '';

  meta = {
    description = "Says what a file shows of itself, for the launcher's chooser of files";
    license = lib.licenses.mit;
    mainProgram = "file-preview";
    platforms = lib.platforms.linux;
  };
})
