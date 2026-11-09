# OpenVINO GenAI as nixpkgs builds it, against nixpkgs' OpenVINO and
# tokenizers, which the public cache holds. Its static Whisper pipeline is
# what runs a whole take on the NPU, decoder and all, and there it takes
# Joel's words only once openvino-genai-npu-prompt.patch lets a prompt in.
#
# Patched, it's in no cache, so every machine compiles it: minutes of C++
# on every core. It asks for big-parallel, the work a builder offered to
# every build takes, so it goes to one while one is there and builds here
# only when none is. OpenVINO itself comes from the cache: nixpkgs builds
# oneDNN into its GPU plugin, without which a language model faults this
# GPU on its first inference.
{
  lib,
  openvino,
  openvino-genai,
}:
# Should nixpkgs leave oneDNN out again, the GPU would fault as before, so
# say so instead.
assert lib.assertMsg (lib.elem (lib.cmakeBool "ENABLE_ONEDNN_FOR_GPU" true) openvino.cmakeFlags)
  "nixpkgs' OpenVINO leaves oneDNN out of its GPU plugin again; build it in";
openvino-genai.overrideAttrs (previous: {
  patches = (previous.patches or [ ]) ++ [ ./openvino-genai-npu-prompt.patch ];
  requiredSystemFeatures = (previous.requiredSystemFeatures or [ ]) ++ [ "big-parallel" ];
})
