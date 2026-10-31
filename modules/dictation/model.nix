# Whisper small, as OpenVINO exported it in fp16 (Apache 2.0): the encoder,
# the stateful decoder, the vocabulary and its merges, the files dictation
# reads, at the revision whose transcripts were measured. The model's and
# its features' configs and the tokenizer's own models are what GenAI's
# pipeline reads besides, to run it whole on the NPU (src/genai.rs).
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
  (file "merges.txt" "sha256-LfKZCjleNejfvHUR4IwS1WAY2NBGkeATPl1jsh4VTcY=")
  (file "config.json" "sha256-LRz59W2us8Gvq7f72d8F6dngDUrKcovgMwl82auOvx8=")
  (file "generation_config.json" "sha256-v2GaQK5l4CiwiRJNBLKUsOiqv7wPn4xbk7WEZS0V7F4=")
  (file "preprocessor_config.json" "sha256-mUg48fpkYsi5s8kO2tqDHxHz3YtGZGNOGPRpTQBcnb8=")
  (file "openvino_tokenizer.xml" "sha256-e8DlxCw4AoLwZbU9ZZeGDfOsUcW+zBRdSLzvm23UI4s=")
  (file "openvino_tokenizer.bin" "sha256-hG88ZfenHxIPznqq9Bs0Lwdn6xPjxOfCpU8dSbXDj9o=")
  (file "openvino_detokenizer.xml" "sha256-q85yOVw6nVT7onC65EZOebE3IbBRFYsEAQwNwYd2RIQ=")
  (file "openvino_detokenizer.bin" "sha256-JULR/mxMXYOOe3xhskmWuUrgy1TmT/bbqrK53V3dfKA=")
]
