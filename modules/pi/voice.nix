# Alan, piper's medium-quality British English voice (from Mycroft's
# mimic3 voices), the model and the config piper finds beside it, at the
# revision that was listened for. Of the male voices measured on one line,
# he spoke lowest and steadiest: a median of 100 Hz, against 103 to 115
# for Norman, Joe, John and the rest and 179 for Ryan, so lowered a
# little more he stays a calm voice rather than a strained one.
{ fetchurl, linkFarm }:
let
  revision = "e7229f7d33a10e145e109b886d661984201ac3c4";
  file = name: hash: {
    inherit name;
    path = fetchurl {
      url = "https://huggingface.co/rhasspy/piper-voices/resolve/${revision}/en/en_GB/alan/medium/${name}";
      inherit hash;
    };
  };
in
linkFarm "piper-voice-en-gb-alan-medium" [
  (file "en_GB-alan-medium.onnx" "sha256-CjCWaJMiBedigB8e/Cc2zUsBIDKWIq32K+CeVjOdMzA=")
  (file "en_GB-alan-medium.onnx.json" "sha256-wPDRJOWJXADnwDs13MgofzGaaZijZbGC3rXI51LujB4=")
]
