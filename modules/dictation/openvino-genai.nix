# OpenVINO GenAI as nixpkgs builds it, with the two fixes from GenAI's main
# that its 2026.4.0 needs to compile as C++20 (openvino-genai-cxx20.patch).
# Its static Whisper pipeline is what runs a whole take on the NPU, decoder
# and all, and there it takes Joel's words only once
# openvino-genai-npu-prompt.patch lets a prompt in.
{ openvino-genai }:
openvino-genai.overrideAttrs (previous: {
  patches = (previous.patches or [ ]) ++ [
    ./openvino-genai-cxx20.patch
    ./openvino-genai-npu-prompt.patch
  ];
})
