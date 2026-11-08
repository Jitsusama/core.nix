# dictation, call-transcripts, screen-text, noise-cancelled and meaning, built
# from this directory's crate with its tests run as it builds, linked to
# OpenVINO's C API and GenAI's, and PipeWire's. It takes what counts as
# a call from modules/calls' crate, so the source holds both, from
# modules, and the build runs in this one. OpenVINO, its tokenizers and
# GenAI are this directory's builds of them (openvino.nix says why), and
# the services name the same tokenizers, from passthru.
{
  callPackage,
  lib,
  pipewire,
  pkg-config,
  rustPlatform,
}:
let
  openvino = callPackage ./openvino.nix { };
  openvino-tokenizers = callPackage ./openvino-tokenizers.nix { inherit openvino; };
  openvino-genai = callPackage ./openvino-genai.nix { inherit openvino openvino-tokenizers; };
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
  # PipeWire's bindings are generated from its headers as they build.
  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
  ];
  buildInputs = [
    openvino
    openvino-genai
    pipewire
  ];
  passthru = { inherit openvino openvino-genai openvino-tokenizers; };
  meta = {
    description = "Dictation, call transcripts, screenshot text and noise-cancelled microphones";
    license = lib.licenses.mit;
    mainProgram = "dictation";
    platforms = lib.platforms.linux;
  };
}
