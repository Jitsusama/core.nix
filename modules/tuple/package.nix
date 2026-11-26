# Tuple's Linux alpha, newer than nixpkgs' v2026_09_16.1: v2026_11_25.0.
# tuple-completes-in-zsh fails unless Tab offers each command core runs
# against it, and each flag tuple-call and tuple-people pass as they
# follow it. That shows they're there, not that they do as before, and no
# check reads the arguments they take. This build calls new and share deprecated, which tuple-call
# runs to open Joel's room and share his screen, so a later one may drop
# them.
# New strings say a camera's or a video's colour space isn't handled, one
# adding that it's converted as BT.601 limited; nobody has yet seen Tuple
# say them. The log lines and stream keys tuple-call reads, the contacts'
# keys and status words tuple-people reads, and the window titles niri's
# rules match are still among this build's strings. That shows they're
# there, not that Tuple says or answers them as before. No check reads
# them, so each bump looks for them by hand among the new build's strings.
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
