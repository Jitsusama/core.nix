# Every model Choose by Ear reads, in one folder: DeepFilterNet 3 as
# model.nix exports it and rewrites it for half precision, which cleans
# each clip, and DNSMOS P.835, which
# scores how each sounds (score.rs says how). DNSMOS is Microsoft's, from
# the DNS Challenge's repository, its code under MIT and the model under
# CC BY 4.0. It's pinned at the commit the trial was measured with.
{
  callPackage,
  fetchurl,
  linkFarm,
}:
let
  deepfilternet = callPackage ./model.nix { };
in
linkFarm "noise-cancelled-models" [
  {
    name = "deepfilternet3-frame-f16.xml";
    path = "${deepfilternet}/deepfilternet3-frame-f16.xml";
  }
  {
    name = "deepfilternet3-frame-f16.bin";
    path = "${deepfilternet}/deepfilternet3-frame-f16.bin";
  }
  {
    name = "dnsmos-sig-bak-ovr.onnx";
    path = fetchurl {
      url = "https://github.com/microsoft/DNS-Challenge/raw/785ab12275d5cf9341c5e946b0eee9f8825ece3b/DNSMOS/DNSMOS/sig_bak_ovr.onnx";
      hash = "sha256-Jp++vbUTqiPN37tZNULsxUAoSpGEmsUFFocOGsePbt0=";
    };
  }
]
