# OpenVINO at 2026.4.1 with oneDNN in its GPU plugin. Without oneDNN a
# language model faults this GPU on its first inference (clFinish's
# CL_OUT_OF_RESOURCES, xe page faults, an engine reset), at 2026.4.0 and
# 2026.4.1 alike; Intel's own 2026.4.1 wheels, which build oneDNN in, run
# the same models on the same kernel and runtime. nixpkgs builds oneDNN in
# since November 2026, but at 2026.4.0, so this keeps the release those
# wheels proved. The CPU and NPU plugins are as nixpkgs builds them.
#
# It's in no cache, so every machine compiles it: the better part of an
# hour of C++ on every core, oneDNN's GPU kernels the most of it. It asks
# for big-parallel, so it goes to a builder while one is there.
{
  fetchFromGitHub,
  lib,
  openvino,
}:
# Should nixpkgs leave oneDNN out again, this release would fault the GPU
# as before, so say so instead.
assert lib.assertMsg (lib.elem (lib.cmakeBool "ENABLE_ONEDNN_FOR_GPU" true) openvino.cmakeFlags)
  "nixpkgs' OpenVINO leaves oneDNN out of its GPU plugin again; build it in here";
openvino.overrideAttrs (previous: {
  version = "2026.4.1";
  src = fetchFromGitHub {
    owner = "openvinotoolkit";
    repo = "openvino";
    tag = "2026.4.1";
    fetchSubmodules = true;
    hash = "sha256-C1qRg0GX+K5tUui66lvK9RzdInIJzXAKyvjkMyB5ce0=";
  };
  requiredSystemFeatures = (previous.requiredSystemFeatures or [ ]) ++ [ "big-parallel" ];
})
