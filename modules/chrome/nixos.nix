# Chrome's open tabs for the launcher, through an extension of this
# folder's (open-tabs/extension), which Chrome installs by itself from a
# local file a preferences file names, as Linux lets it, unasked. Chrome
# comes from wherever the machine gets it, so this installs only the
# extension; a machine with Chrome imports it beside the account's Chrome
# module (home.nix).
#
# The key fixes the extension's id, eefbjjlcekpocpoiobhilnmpabjhmjhm, which
# Chrome takes from the key's public half; it signs this extension alone
# and guards nothing, so it lives here in the open. Chrome reads the
# preferences file only from /usr/share/google-chrome/extensions, which is
# made here, since NixOS has no /usr/share.
{ lib, pkgs, ... }:
let
  packed = pkgs.runCommandLocal "chrome-open-tabs-extension" { } ''
    mkdir $out
    id=$(${lib.getExe (pkgs.callPackage ./pack-extension/package.nix { })} \
      ${./open-tabs/extension} ${./open-tabs/key.pem} $out/open-tabs.crx)
    [ "$id" = eefbjjlcekpocpoiobhilnmpabjhmjhm ] || {
      echo "the key packs as $id, not the id the extension is known by"
      exit 1
    }
  '';
  inherit (lib.importJSON ./open-tabs/extension/manifest.json) version;
  preferences = pkgs.writeText "eefbjjlcekpocpoiobhilnmpabjhmjhm.json" (
    builtins.toJSON {
      external_crx = "${packed}/open-tabs.crx";
      external_version = version;
    }
  );
in
{
  systemd.tmpfiles.rules = [
    "d /usr/share/google-chrome/extensions 0755 root root -"
    "L+ /usr/share/google-chrome/extensions/eefbjjlcekpocpoiobhilnmpabjhmjhm.json - - - - ${preferences}"
  ];
}
