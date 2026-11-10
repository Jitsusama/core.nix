# A stand-in for Apple's restore image: a zip under Apple's name holding
# one member stored uncompressed, as Apple's are, built into its partition
# by the same apple-restore.nix a machine repository's stick takes.
#
# The zip is the same bytes wherever it's built. A zip keeps its member's
# time, which was the moment of the build, so the copy inside a stick image
# built on one builder differed from the copy the test compared it with,
# built on another, and the written stick was taken for one that lost bytes.
{ pkgs }:
let
  zip =
    pkgs.runCommand "UniversalMac_27.2_26B123_Restore.ipsw" { nativeBuildInputs = [ pkgs.zip ]; }
      ''
        echo "a stand-in for Apple's build manifest" > BuildManifest.plist
        # The earliest time a zip can say, in the time zone it says it in.
        export TZ=UTC
        touch -d 1980-01-01T00:00:00 BuildManifest.plist
        zip -0 -X -q restore.zip BuildManifest.plist
        cp restore.zip "$out"
      '';
in
pkgs.callPackage ../../../modules/guide/apple-restore.nix { } {
  url = "https://updates.cdn-apple.com/stand-in/UniversalMac_27.2_26B123_Restore.ipsw";
  inherit zip;
}
