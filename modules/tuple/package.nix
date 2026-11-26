# Tuple's Linux alpha, newer than nixpkgs' v2026_09_16.1: v2026_11_25.0,
# whose command is the one Tuple's docs describe (tuple call, contacts,
# state, webcam). v2026_11_19.0 was the first to share a webcam from
# Linux; this one checks a camera's colour space and says when Tuple
# can't take it. The log lines and stream keys tuple-call reads, and the
# contacts' keys and status words tuple-people reads, are in both builds.
# Screen sharing through the portal was last seen working here on
# v2026_10_21.0. Webcam sharing is yet to be seen here: the build reads
# cameras through PipeWire, where the IR camera is hidden, and until the
# Brio is shared through a relay every app reads, whichever app opens it
# first keeps it from the rest, a call in Meet included.
# The address and checksum of each build are what Tuple's latest_version
# endpoint gives, the one nixpkgs' own update script reads.
#
# nixpkgs writes the command's completions through its wrapper, which runs
# the command as .tuple-wrapped, so the zsh completion it writes is for
# that name, which zsh never completes. This names it for tuple, and stays
# while nixpkgs' completion does.
{
  stdenv,
  fetchurl,
  tuple,
}:
let
  release = "v2026_11_25.0";
  builds = {
    x86_64-linux = {
      arch = "x86_64";
      hash = "sha256-HkQfchyjUtQvxfKfCN0rqiijO/U89SHuob1bn2dBveg=";
    };
    aarch64-linux = {
      arch = "arm64";
      hash = "sha256-mmrhx5ECwoYtVbHevmVyKgBytAXGvjxD3Vo0g3RQcvY=";
    };
  };
  build = builds.${stdenv.hostPlatform.system} or null;
in
if build == null then
  tuple
else
  tuple.overrideAttrs {
    version = "0-unstable-2026-11-25";
    src = fetchurl {
      url = "https://tuple-client-releases.s3.amazonaws.com/linux/${release}/${build.arch}/tuple";
      inherit (build) hash;
    };
    postInstall = ''
      sed -i 's/\.tuple-wrapped/tuple/g' $out/share/zsh/site-functions/_tuple
      grep -q '^#compdef tuple$' $out/share/zsh/site-functions/_tuple
    '';
  }
