# Parakeet TDT 0.6B v3, NVIDIA's speech model (CC BY 4.0, by NVIDIA), as
# FluidInference exported it for OpenVINO: the spectrogram, the encoder,
# the decoder and the joint network, each an IR, and the v3 vocabulary, at
# the revision whose transcripts were measured. call-transcripts hears
# through it (src/parakeet.rs); dictation keeps Whisper (model.nix).
{ fetchurl, linkFarm }:
let
  revision = "b299728668074299b2d19208d30d28460ad81adb";
  file = name: hash: {
    inherit name;
    path = fetchurl {
      url = "https://huggingface.co/FluidInference/parakeet-tdt-0.6b-v3-ov/resolve/${revision}/${name}";
      inherit hash;
    };
  };
in
linkFarm "parakeet-tdt-0.6b-v3-ov" [
  (file "parakeet_melspectogram.xml" "sha256-cgpvhexu1AVxNmr1M11VSX7QbutDd1KEBoScozQXb40=")
  (file "parakeet_melspectogram.bin" "sha256-ogwymfhjsDzLC5FGX9AvHV7d10m8REGONnTnpriLERY=")
  (file "parakeet_encoder.xml" "sha256-vHNp7yPxXi9ABJyAhBnwzzOplvVG9njzBY6ymx1UNEs=")
  (file "parakeet_encoder.bin" "sha256-UpmeTtFZVHCoC54GZt3PmUjAs29DeWfLMZxcMywT6I4=")
  (file "parakeet_decoder.xml" "sha256-Vve4LNe5LwFfAV8SfAMkShOxV/P++d2TGujlmPqJLlY=")
  (file "parakeet_decoder.bin" "sha256-0y1wPu0EXzhWQ2Kz8HokBf8nSxkgAbkIjTY7e4Mu340=")
  (file "parakeet_joint.xml" "sha256-QO7WcP7KMXAxRQ+XAPOBOy59PWNHgKbKQY7l0JyKtWw=")
  (file "parakeet_joint.bin" "sha256-6I8pitjKjZD/ULAUcoGjyoz2d1UGGVE8ABOjFgtX8AE=")
  (file "parakeet_v3_vocab.json" "sha256-qoQfesDcM/HQZuiMHxu4eaBsyOzit3QntJRisK/VodY=")
]
