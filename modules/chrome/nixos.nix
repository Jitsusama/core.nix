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
#
# The extension answers chrome-open-tabs (open-tabs/host), its
# native-messaging host, which Chrome starts from the file named for the
# host in /etc/opt/chrome/native-messaging-hosts, allowed to this extension
# alone. A managed Chrome may refuse every extension its policy doesn't
# list, so the launcher reads Chrome's own session file instead (home.nix),
# which gives every tab but can't go to one or close it; the host, on the
# path, is what a Chrome that allows the extension would answer through.
{ lib, pkgs, ... }:
let
  host = pkgs.callPackage ./open-tabs/host/package.nix { };
  # The host as Chrome finds it, allowed to our extension alone.
  hosting = builtins.toJSON {
    name = "jitsusama.chrome_open_tabs";
    description = "Chrome's open tabs, for the launcher";
    path = lib.getExe host;
    type = "stdio";
    allowed_origins = [ "chrome-extension://eefbjjlcekpocpoiobhilnmpabjhmjhm/" ];
  };
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
  environment.systemPackages = [ host ];

  environment.etc."opt/chrome/native-messaging-hosts/jitsusama.chrome_open_tabs.json".text = hosting;

  systemd.tmpfiles.rules = [
    "d /usr/share/google-chrome/extensions 0755 root root -"
    "L+ /usr/share/google-chrome/extensions/eefbjjlcekpocpoiobhilnmpabjhmjhm.json - - - - ${preferences}"
  ];
}
