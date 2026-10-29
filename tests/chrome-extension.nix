# Chrome installs the launcher's open-tabs extension by itself from the
# local file the Chrome module names (modules/chrome/nixos.nix), asking
# nobody. Google Chrome isn't free, so Chromium stands in: it installs an
# extension named in a preferences file the same way, from the system's
# share/chromium/extensions, where nixpkgs' Chromium looks rather than
# google-chrome's /usr/share, and where the module's own file is linked.
# Chromium starts without a screen, and the first case passes once it has
# unpacked the extension into its profile with its two permissions granted.
# The second opens two pages and asks chrome-open-tabs, the extension's
# host, to list them, bring one to the front and close the other, as the
# launcher will. Chromium without a screen opens one page alone, so that
# case gives it a virtual X screen. The script is chrome-extension-vm-test.py.
{ self, pkgs }:
let
  id = "eefbjjlcekpocpoiobhilnmpabjhmjhm";
  values = pkgs.writeText "chrome-extension-values.json" (builtins.toJSON { inherit id; });
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "chrome-extension";

  nodes.machine = {
    imports = [ self.nixosModules.chrome ];

    # Its user manager keeps a runtime folder for the host's socket.
    users.users.joel = {
      isNormalUser = true;
      linger = true;
    };
    environment.etc."chromium/native-messaging-hosts/jitsusama.chrome_open_tabs.json".source =
      "/etc/opt/chrome/native-messaging-hosts/jitsusama.chrome_open_tabs.json";
    environment.systemPackages = [
      pkgs.chromium
      pkgs.xvfb-run
      (pkgs.runCommandLocal "chromium-takes-the-open-tabs-extension" { } ''
        mkdir -p $out/share/chromium/extensions
        ln -s /usr/share/google-chrome/extensions/${id}.json $out/share/chromium/extensions/
      '')
    ];
    environment.pathsToLink = [ "/share/chromium" ];
    virtualisation.memorySize = 2048;
  };

  tests = ./chrome-extension-vm-test.py;
  inherit values;
}
