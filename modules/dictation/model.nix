# Whisper small, as OpenVINO exported it in fp16 (Apache 2.0): the encoder,
# the stateful decoder and the vocabulary, the files dictation reads, at
# the revision whose transcripts were measured.
{ fetchurl, linkFarm }:
let
  revision = "2410d022171ca8a97343182f88eec8807a324db9";
  file = name: hash: {
    inherit name;
    path = fetchurl {
      url = "https://huggingface.co/OpenVINO/whisper-small-fp16-ov/resolve/${revision}/${name}";
      inherit hash;
    };
  };
in
linkFarm "whisper-small-fp16-ov" [
  (file "openvino_encoder_model.xml" "sha256-a+AX/m1wDbyscdvIiAaK+p3PrwqqwXOCMU+W2H+WWgI=")
  (file "openvino_encoder_model.bin" "sha256-dxwvCLxo9R0Z0MHlzhMRHtmCin5HXfmO3bQEPddW8/I=")
  (file "openvino_decoder_model.xml" "sha256-cD3ZfbuaF1mQsEmAmcL6FaaG8kQtZgVyZgBvjl/QLCc=")
  (file "openvino_decoder_model.bin" "sha256-Mc5QKQ6Uv31Z6QEVFMxi/I0kYhAmoaupmCwCqLohfik=")
  (file "vocab.json" "sha256-j2gLujGeAaZT0uil28F6kVcXngV25s50zgwGNWxuJPk=")
]
