# Phi-3.5-mini-instruct, as OpenVINO exported it with its weights in int4
# by channel (MIT), the layout the NPU's static LLM pipeline runs: the
# model, its tokenizer and detokenizer (whose rt_info carries the chat
# template) and the configs GenAI's pipeline reads, at the revision whose
# summaries were measured on the NPU (about 8 s for a short call, 1 to 2 s
# of CPU time).
{ fetchurl, linkFarm }:
let
  revision = "dc27b4e451118b1e82c159a839476df02797837a";
  file = name: hash: {
    inherit name;
    path = fetchurl {
      url = "https://huggingface.co/OpenVINO/Phi-3.5-mini-instruct-int4-cw-ov/resolve/${revision}/${name}";
      inherit hash;
    };
  };
in
linkFarm "phi-3.5-mini-instruct-int4-cw-ov" [
  (file "openvino_model.xml" "sha256-uN3OiIudzcPT51zx02e2hnVzroh8QQdGJerpQzn0OBE=")
  (file "openvino_model.bin" "sha256-mtNnDpr19P3bcOg5SIKrpGb24BKtPFjZ/OKt59hmJf0=")
  (file "openvino_tokenizer.xml" "sha256-9se/dQkp/mu9nnP3yFPgoqOKJH9OC9CcucswCXy1lZM=")
  (file "openvino_tokenizer.bin" "sha256-aws9kd5nDTADhDggh0QoyFGxn14V7qp/eMi5Xrf9Ff4=")
  (file "openvino_detokenizer.xml" "sha256-1g0lroACUlln5qXWI40NWXfPQ4zdAEIJPImKkm7Zqh8=")
  (file "openvino_detokenizer.bin" "sha256-sWhzN2A23TRPDKO0vkP9EyQ6CrcI6vp1YKMBUxCTWG8=")
  (file "config.json" "sha256-zP0hKcSbowG5IbYnSmY61U8IO5j8OH7JDUNA/4Y+jq4=")
  (file "generation_config.json" "sha256-Qv2y+d/GtR+x3+u3UExPHTR6GV5K4mpavq6ytFszVRE=")
  (file "openvino_config.json" "sha256-tk4frcFFJM2OIzLxZbu78ch+qIWPZVLqWpDxOmjxdTQ=")
  (file "tokenizer_config.json" "sha256-ePFpn2crpcjJWPqfaQQFYKAxCWfbiPy6gHRtKZc48Tw=")
  (file "special_tokens_map.json" "sha256-R01plnfiZO3ThRVZVIy7thgBrY2msknIFFUAdgIxPDQ=")
  (file "added_tokens.json" "sha256-X1yU0wkB/pyWHXdtUerb2kRUmRp3tT024L4FcajnKn0=")
  (file "chat_template.jinja" "sha256-eNl2pEK83i8L4Vqvu44wUOEQT4ZzImb2hAMlG4mYKpA=")
]
