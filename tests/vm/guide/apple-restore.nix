# A stand-in for Apple's restore image: a zip under Apple's name holding
# one member stored uncompressed, as Apple's are, built into its partition
# by the same apple-restore.nix a machine repository's stick takes.
{ pkgs }:
let
  zip =
    pkgs.runCommand "UniversalMac_27.2_26B123_Restore.ipsw" { nativeBuildInputs = [ pkgs.zip ]; }
      ''
        echo "a stand-in for Apple's build manifest" > BuildManifest.plist
        zip -0 -X -q restore.zip BuildManifest.plist
        cp restore.zip "$out"
      '';
in
pkgs.callPackage ../../../modules/guide/apple-restore.nix { } {
  url = "https://updates.cdn-apple.com/stand-in/UniversalMac_27.2_26B123_Restore.ipsw";
  inherit zip;
}
