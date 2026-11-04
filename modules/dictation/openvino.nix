# OpenVINO at 2026.4.1 with oneDNN in its GPU plugin, which nixpkgs
# leaves out. Without it a language model faults this GPU on its first
# inference (clFinish's CL_OUT_OF_RESOURCES, xe page faults, an engine
# reset), at 2026.4.0 and 2026.4.1 alike; Intel's own 2026.4.1 wheels,
# which build oneDNN in, run the same models on the same kernel and
# runtime. The CPU and NPU plugins are as nixpkgs builds them.
#
# It's in no cache, so every machine compiles it: the better part of an
# hour of C++ on every core, oneDNN's GPU kernels the most of it. It asks
# for big-parallel, so it goes to a builder while one is there.
{
  fetchFromGitHub,
  lib,
  openvino,
}:
let
  withoutOneDnn = lib.cmakeBool "ENABLE_ONEDNN_FOR_GPU" false;
in
# Should nixpkgs stop naming the flag, swapping it would quietly do
# nothing, so say so instead.
assert lib.assertMsg (lib.elem withoutOneDnn openvino.cmakeFlags)
  "nixpkgs' OpenVINO no longer leaves oneDNN out of its GPU plugin; drop the swap";
openvino.overrideAttrs (previous: {
  version = "2026.4.1";
  src = fetchFromGitHub {
    owner = "openvinotoolkit";
    repo = "openvino";
    tag = "2026.4.1";
    fetchSubmodules = true;
    hash = "sha256-C1qRg0GX+K5tUui66lvK9RzdInIJzXAKyvjkMyB5ce0=";
  };
  cmakeFlags = map (
    flag: if flag == withoutOneDnn then lib.cmakeBool "ENABLE_ONEDNN_FOR_GPU" true else flag
  ) previous.cmakeFlags;
  requiredSystemFeatures = (previous.requiredSystemFeatures or [ ]) ++ [ "big-parallel" ];
})
