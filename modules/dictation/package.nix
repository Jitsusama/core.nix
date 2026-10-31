# dictation, call-transcripts and screen-text, built from this
# directory's crate with its tests run as it builds, linked to OpenVINO's
# C API and GenAI's. It takes what counts as a call from modules/calls'
# crate, so the source holds both, from modules, and the build runs in
# this one.
{
  callPackage,
  lib,
  rustPlatform,
  openvino,
}:
let
  openvino-genai = callPackage ./openvino-genai.nix { };
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../calls/Cargo.toml
      ../calls/src
    ];
  };
in
rustPlatform.buildRustPackage {
  pname = "dictation";
  version = "0.1.0";
  inherit src;
  sourceRoot = "${src.name}/dictation";
  cargoLock.lockFile = ./Cargo.lock;
  buildInputs = [
    openvino
    openvino-genai
  ];
  passthru = { inherit openvino-genai; };
  meta = {
    description = "Dictation, call transcripts and screenshot text, on the NPU";
    license = lib.licenses.mit;
    mainProgram = "dictation";
    platforms = lib.platforms.linux;
  };
}
