# Google's siglip2-base-patch16-224 (Apache-2.0), as onnx-community
# exported its two towers at sixteen bits, with its tokenizer, at the
# revision measured on an Intel NPU: the picture tower takes a median
# 19 ms a picture at 224 pixels a side, the text tower 11.5 ms for what's
# typed at 64 tokens, each about 7 ms of CPU time. Their eight-bit text
# tower runs there too, but lands every picture 0.03 to 0.06 further from
# the words, losing a keyboard and code it found, so sixteen it is. The
# folder holds what meaning reads for pictures (sight.rs): text.onnx,
# vision.onnx and tokenizer.json.
{ fetchurl, runCommand }:
let
  revision = "ba1f3b0843f24bc5417d38e19c37b287d719b2f4";
  file =
    name: hash:
    fetchurl {
      url = "https://huggingface.co/onnx-community/siglip2-base-patch16-224-ONNX/resolve/${revision}/${name}";
      inherit hash;
    };
  text = file "onnx/text_model_fp16.onnx" "sha256-cR2latoKSqEcfdMyDfdBCBo8rk8K4bXlxtWylHONDrA=";
  vision = file "onnx/vision_model_fp16.onnx" "sha256-oZWfe9OZOmB+SIOfbQHiW4dv52r9owGwKLeO72iqvZU=";
  tokenizer = file "tokenizer.json" "sha256-y5FA+uOsUSLJctN634PhJIRxo4FHrXb4IVyIcsb9gyI=";
in
runCommand "siglip2-base-patch16-224-fp16" { } ''
  mkdir $out
  ln -s ${text} $out/text.onnx
  ln -s ${vision} $out/vision.onnx
  ln -s ${tokenizer} $out/tokenizer.json
''
