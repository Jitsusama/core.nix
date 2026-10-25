# Tuple's Linux alpha, newer than nixpkgs' v2026_09_16.1: v2026_10_23.0,
# whose command is the one Tuple's docs describe (tuple call, contacts,
# state), its every command's help the same as v2026_10_21.0's, whose
# screen sharing through the portal was seen to work here.
# The address and checksum of each build are what Tuple's latest_version
# endpoint gives, the one nixpkgs' own update script reads. Once nixpkgs'
# tuple is this new, this file can go.
{
  stdenv,
  fetchurl,
  tuple,
}:
let
  release = "v2026_10_23.0";
  builds = {
    x86_64-linux = {
      arch = "x86_64";
      hash = "sha256-0mYRG8QdXgWS2ExbhOXOjOi/pB16IxahO4lXOJ/Sbis=";
    };
    aarch64-linux = {
      arch = "arm64";
      hash = "sha256-ySj7s1L18TszOr33RWV/0xnYQphq4Uuae24BENfKdFo=";
    };
  };
  build = builds.${stdenv.hostPlatform.system} or null;
in
if build == null then
  tuple
else
  tuple.overrideAttrs {
    version = "0-unstable-2026-10-23";
    src = fetchurl {
      url = "https://tuple-client-releases.s3.amazonaws.com/linux/${release}/${build.arch}/tuple";
      inherit (build) hash;
    };
  }
