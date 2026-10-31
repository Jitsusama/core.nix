# OpenVINO GenAI as nixpkgs builds it, with the two fixes from GenAI's main
# that its 2026.4.0 needs to compile as C++20 (openvino-genai-cxx20.patch).
# Its static Whisper pipeline is what runs a whole take on the NPU, decoder
# and all, and there it takes Joel's words only once
# openvino-genai-npu-prompt.patch lets a prompt in.
#
# Patched, it's in no cache, so every machine compiles it: minutes of C++
# on every core. It asks for big-parallel, the work a builder offered to
# every build takes, so it goes to one while one is there and builds here
# only when none is.
{ openvino-genai }:
openvino-genai.overrideAttrs (previous: {
  patches = (previous.patches or [ ]) ++ [
    ./openvino-genai-cxx20.patch
    ./openvino-genai-npu-prompt.patch
  ];
  requiredSystemFeatures = (previous.requiredSystemFeatures or [ ]) ++ [ "big-parallel" ];
})
