# dictation and call-transcripts, built from this directory's crate with
# its tests run as it builds, linked to OpenVINO's C API and GenAI's.
{
  callPackage,
  lib,
  rustPlatform,
  openvino,
}:
let
  openvino-genai = callPackage ./openvino-genai.nix { };
in
rustPlatform.buildRustPackage {
  pname = "dictation";
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
  buildInputs = [
    openvino
    openvino-genai
  ];
  passthru = { inherit openvino-genai; };
  meta = {
    description = "Dictation and call transcripts, Whisper on the NPU";
    license = lib.licenses.mit;
    mainProgram = "dictation";
    platforms = lib.platforms.linux;
  };
}
