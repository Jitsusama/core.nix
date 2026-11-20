# TitaNet small, NVIDIA's speaker-embedding model (under the NeMo
# Toolkit's licence, Apache 2.0, by NVIDIA), as sherpa-onnx exported it to
# ONNX with its features' settings kept in the file. call-transcripts tells
# the far end's voices apart with it (src/voices.rs): one embedding a take,
# grouped at the call's end. On three AMI meetings it kept its voices apart
# where WeSpeaker's ResNet34 mixed one of them up half the time.
{ fetchurl }:
fetchurl {
  name = "nemo_en_titanet_small.onnx";
  url = "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/nemo_en_titanet_small.onnx";
  hash = "sha256-rUoYAkhdizTHItKp0EJJZi8uzl0op6A5BjyiL1FaeJ4=";
}
