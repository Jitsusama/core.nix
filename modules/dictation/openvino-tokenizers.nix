# OpenVINO Tokenizers at the release that matches openvino.nix's
# OpenVINO, built against it. nixpkgs' patch, which takes PCRE2 from the
# system rather than downloading it, names the PCRE2 its 2026.4.0 fetched;
# 2026.4.1.0 fetches 10.48, so openvino-tokenizers-system-pcre2.patch is
# the same patch rebased onto those lines.
{
  fetchFromGitHub,
  openvino,
  openvino-tokenizers,
}:
(openvino-tokenizers.override { inherit openvino; }).overrideAttrs {
  version = "2026.4.1.0";
  src = fetchFromGitHub {
    owner = "openvinotoolkit";
    repo = "openvino_tokenizers";
    tag = "2026.4.1.0";
    hash = "sha256-1mTW25PAAC81kAUgdnIrXH011imSxD+RyWIcrwoqq6g=";
  };
  patches = [ ./openvino-tokenizers-system-pcre2.patch ];
}
