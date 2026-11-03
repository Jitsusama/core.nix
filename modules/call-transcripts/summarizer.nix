# Gemma 4 26B-A4B, Google's mixture of experts with about 4B of its 26B
# parameters at work for each token (Apache 2.0), as OpenVINO exported it
# with its weights in int4: the language model, its embeddings, the
# vision encoder its pipeline loads though no picture is given, the
# tokenizer and detokenizer, and the configs GenAI reads, at the revision
# whose quotes were checked on Joel's calls (2026-11-03). It runs on the
# GPU, taking about 16 GB, through GenAI's VLM pipeline (dictation's
# language.rs says why). call-transcripts picks a call's quotes with it.
{ fetchurl, linkFarm }:
let
  revision = "4e5885c3f03844f5887982def09708bdc9f53f7a";
  file = name: hash: {
    inherit name;
    path = fetchurl {
      url = "https://huggingface.co/OpenVINO/gemma-4-26b-a4b-it-int4-ov/resolve/${revision}/${name}";
      inherit hash;
    };
  };
in
linkFarm "gemma-4-26b-a4b-it-int4-ov" [
  (file "openvino_language_model.xml" "sha256-qWFL9CO+SeTfrSo7sbAp2PeAu1W6h/f+a13rT7rN5hY=")
  (file "openvino_language_model.bin" "sha256-D4hxMK72NUXWVeoq3athaGgO/ptlMcvEbC4vxZmsir4=")
  (file "openvino_text_embeddings_model.xml" "sha256-Y/RQ6a+ZGktsYQsqUBXwOS3gNs683qELCDXT1wyuQ9M=")
  (file "openvino_text_embeddings_model.bin" "sha256-LY373MIXSqYiNiFQLcVm2eiBt8PbP/Lh6WplzueCfts=")
  (file "openvino_text_embeddings_per_layer_model.xml" "sha256-t2zSBod90aRo2K0ctXbfE2y9HOvuXj3s00cj9yrzYUs=")
  (file "openvino_text_embeddings_per_layer_model.bin" "sha256-V/2nqwNAzacYGaPs1fnlLjRiYP6ED93MnIfTYUtagVs=")
  (file "openvino_vision_embeddings_model.xml" "sha256-WKzQh7iXhvj4CN+dghMhxuez/bJdrSyLpqzgpPy+9vM=")
  (file "openvino_vision_embeddings_model.bin" "sha256-E8nY/ze+lZLgboqfZeKwR73Gsk1byQXAIuTRiZazw4I=")
  (file "openvino_tokenizer.xml" "sha256-59pYYRNMj2ywsbfOU/+R1qu0Rg3742ItD9JOfKE9zvU=")
  (file "openvino_tokenizer.bin" "sha256-csxhOFoULCmh6jcSW516TFd1/oOGpfbbHdEQjqXsE2E=")
  (file "openvino_detokenizer.xml" "sha256-Y/ievm7dFcM3LXnVeiqkZUqeDyyUnADD7ET7A7aUWU4=")
  (file "openvino_detokenizer.bin" "sha256-RhkZvae1RkU+kDBSWIxXjj6U6eghW3zsB8sAy5dFnRM=")
  (file "config.json" "sha256-nRVoTC2dUjuO4MLhddHdoXTSkMW18fl+6jwpaff86YY=")
  (file "generation_config.json" "sha256-TC0YwHLcVMwu6PSinmNSX1a8l6FKDpPJowdVuntrkBQ=")
  (file "openvino_config.json" "sha256-xtFlaQ5o7P8JOzVGTg5W8yC1C6TINP0pVvXebU2U0d0=")
  (file "preprocessor_config.json" "sha256-EkdtYmuJ4nOpssakk6YxmrAkJ+v802NdkrKVhLJ8ckk=")
  (file "processor_config.json" "sha256-Mr30XSrUzCmggi3dFXoYLedmRPBBmmIo0VFJUlbpgTw=")
  (file "tokenizer_config.json" "sha256-N1b3tsNTpab3QDd8/es4Aqphww7VGoPr0q7vCXHauj8=")
  (file "tokenizer.json" "sha256-zI06DONkZszBJ4v5h99fcdsXGbnKa0EYJk9Fy2J7/g8=")
  (file "chat_template.jinja" "sha256-rlNGS/O+JYArOls33vf9iWZwZ9dXcEmzstdMTY3kxtQ=")
]
