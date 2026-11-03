# OpenVINO GenAI at the release that matches openvino.nix's OpenVINO,
# built against it and openvino-tokenizers.nix, with the two fixes from
# GenAI's main that 2026.4.1.0 still needs to compile as C++20
# (openvino-genai-cxx20.patch). Its static Whisper pipeline is what runs a
# whole take on the NPU, decoder and all, and there it takes Joel's words
# only once openvino-genai-npu-prompt.patch lets a prompt in.
#
# Patched, it's in no cache, so every machine compiles it: minutes of C++
# on every core. It asks for big-parallel, the work a builder offered to
# every build takes, so it goes to one while one is there and builds here
# only when none is.
{
  fetchFromGitHub,
  openvino,
  openvino-genai,
  openvino-tokenizers,
}:
(openvino-genai.override { inherit openvino openvino-tokenizers; }).overrideAttrs (previous: {
  version = "2026.4.1.0";
  src = fetchFromGitHub {
    owner = "openvinotoolkit";
    repo = "openvino.genai";
    tag = "2026.4.1.0";
    hash = "sha256-NgfzcBl3DPja2Jcf+9VFGsTQG1pKrJsB1FlbGpjY/bc=";
  };
  patches = (previous.patches or [ ]) ++ [
    ./openvino-genai-cxx20.patch
    ./openvino-genai-npu-prompt.patch
  ];
  requiredSystemFeatures = (previous.requiredSystemFeatures or [ ]) ++ [ "big-parallel" ];
})
