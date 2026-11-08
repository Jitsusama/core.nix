# BAAI's bge-base-en-v1.5 (MIT), as OpenVINO exported it at eight bits
# with its tokenizer, at the revision measured on optimus's NPU: 4.7 to
# 5 ms a query at 64 tokens using about 1 ms of CPU time, against 27 ms
# on the processor (M4RL61's npu-everywhere, item 7). The folder holds
# what meaning reads: the model's graph and weights, and the tokenizer.
{ fetchurl, runCommand }:
let
  revision = "9a76c2f2cf23bb2fd4a20469a5170cb7ebe0d5d2";
  file =
    name: hash:
    fetchurl {
      url = "https://huggingface.co/OpenVINO/bge-base-en-v1.5-int8-ov/resolve/${revision}/${name}";
      inherit hash;
    };
  graph = file "openvino_model.xml" "sha256-8WYHcB94lYiS8JnXmfhDwWWNTByFO7nr7f9ljwfcbGs=";
  weights = file "openvino_model.bin" "sha256-bvMcMwMaAVwj3y5o9kMVApOek7ysxJx+oIF0YqSZ1dM=";
  tokenizer = file "tokenizer.json" "sha256-0kGmDV6PBMwbKz6e96SSGye/Um2fYFCrkPkmeh+eXGY=";
in
runCommand "bge-base-en-v1.5-int8" { } ''
  mkdir $out
  ln -s ${graph} $out/openvino_model.xml
  ln -s ${weights} $out/openvino_model.bin
  ln -s ${tokenizer} $out/tokenizer.json
''
