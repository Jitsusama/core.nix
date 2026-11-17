# Tuple's Linux alpha, newer than nixpkgs' v2026_09_16.1: v2026_11_16.0,
# whose command is the one Tuple's docs describe (tuple call, contacts,
# state). Every command v2026_10_23.0 had answers its help the same, but
# for call react, which takes emoji and animations beside its sounds. New
# commands raise a hand, share cards and download the transcription
# model. The log lines and stream keys tuple-call reads are in both
# builds. Screen sharing through the portal was last seen working here on
# v2026_10_21.0.
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
  release = "v2026_11_16.0";
  builds = {
    x86_64-linux = {
      arch = "x86_64";
      hash = "sha256-YL9OgRIe7m0XwWrs8BZvxW7VVY564dPHL73Alz7xLEw=";
    };
    aarch64-linux = {
      arch = "arm64";
      hash = "sha256-2bOOKHgUmSqA9eAwAYKB/EQY4b1HB0MW6yvC1EAJQd8=";
    };
  };
  build = builds.${stdenv.hostPlatform.system} or null;
in
if build == null then
  tuple
else
  tuple.overrideAttrs {
    version = "0-unstable-2026-11-16";
    src = fetchurl {
      url = "https://tuple-client-releases.s3.amazonaws.com/linux/${release}/${build.arch}/tuple";
      inherit (build) hash;
    };
    postInstall = ''
      sed -i 's/\.tuple-wrapped/tuple/g' $out/share/zsh/site-functions/_tuple
      grep -q '^#compdef tuple$' $out/share/zsh/site-functions/_tuple
    '';
  }
