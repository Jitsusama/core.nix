# PaddleOCR's PP-OCRv5 mobile text detector and English recognizer, as
# PaddlePaddle exported them to ONNX (Apache-2.0), at the revisions whose
# speed was measured on the NPU: 6.2 ms to find the text in a 640x640
# tile and 1.8 ms to read a line, under half a millisecond of CPU time
# each. The characters the recognizer knows are taken from its own config,
# one a line, the folder screen-text reads.
{
  fetchurl,
  runCommand,
  yq-go,
}:
let
  file =
    repo: revision: name: hash:
    fetchurl {
      url = "https://huggingface.co/PaddlePaddle/${repo}/resolve/${revision}/${name}";
      inherit hash;
    };
  detector = file "PP-OCRv5_mobile_det_onnx" "e6f4fa85f00e168c862bc462aebca69eef9b3d3d";
  recognizer = file "en_PP-OCRv5_mobile_rec_onnx" "3fafbc3b5dcf93dd72add9f48368be8a3a2cd33b";
  finds = detector "inference.onnx" "sha256-pDGYVlnckhl0F3qVrc+7kP2eUZiaXgTXDQt19Ze25h0=";
  reads = recognizer "inference.onnx" "sha256-tfgz38XQ63HaOXtO+gbr7um0MbaQpH1q9A132Oq8VX8=";
  config = recognizer "inference.yml" "sha256-J+kdBYL0AWiqIYMDx24YS8ePp6XRBarQz7rYRYtEEGc=";
in
runCommand "pp-ocrv5-mobile-en" { nativeBuildInputs = [ yq-go ]; } ''
  mkdir $out
  ln -s ${finds} $out/det.onnx
  ln -s ${reads} $out/rec.onnx
  yq -r '.PostProcess.character_dict[]' ${config} > $out/characters.txt
  # Its config lists 436; any other count is a config read wrong, which
  # would shift every character the recognizer gives.
  test "$(wc -l < $out/characters.txt)" = 436
''
