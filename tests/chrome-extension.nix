# Chrome installs the launcher's open-tabs extension by itself from the
# local file the Chrome module names (modules/chrome/nixos.nix), asking
# nobody. Google Chrome isn't free, so Chromium stands in: it installs an
# extension named in a preferences file the same way, from the system's
# share/chromium/extensions, where nixpkgs' Chromium looks rather than
# google-chrome's /usr/share, and where the module's own file is linked.
# Chromium starts without a screen, and the first case passes once it has
# unpacked the extension into its profile with its two permissions granted.
# The second opens two pages and asks the extension's host to list them,
# as the programs daemon asks its socket where the account's Chrome source
# says it's there, then, through the commands that source gives the
# launcher (modules/chrome/home.nix), which find it by the file this module
# writes for Chrome, brings one to the front and closes the other, as the
# launcher does. Chromium without a screen opens one page alone, so that
# case gives it a virtual X screen. The third needs no extension: Chromium
# opens two pages served here, and chrome-session reads both, in one
# window with one in front, from the session file Chromium writes, as the
# programs daemon reads Chrome's. The script is chrome-extension-vm-test.py.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  id = "eefbjjlcekpocpoiobhilnmpabjhmjhm";
  # The extension's id, and Chrome's source of what it has open, as the
  # account's launcher is given it.
  values =
    { nodes }:
    pkgs.writeText "chrome-extension-values.json" (
      builtins.toJSON {
        inherit id;
        source = lib.findFirst (
          source: source.app or null == "google-chrome"
        ) null nodes.machine.home-manager.users.joel.jitsusama.quickshell.launcher.recents;
      }
    );
  pages = pkgs.runCommandLocal "chrome-session-pages" { } ''
    mkdir $out
    echo '<title>One</title>' > $out/one.html
    echo '<title>Two</title>' > $out/two.html
  '';
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "chrome-extension";

  nodes.machine = {
    imports = [
      self.nixosModules.chrome
      self.nixosModules.home-manager
    ];
    home-manager.users.joel = {
      imports = [ self.homeModules.chrome ];
      home.stateVersion = "26.05";
    };

    # Its user manager keeps a runtime folder for the host's socket.
    users.users.joel = {
      isNormalUser = true;
      linger = true;
    };
    environment.etc."chromium/native-messaging-hosts/jitsusama.chrome_open_tabs.json".source =
      "/etc/opt/chrome/native-messaging-hosts/jitsusama.chrome_open_tabs.json";
    environment.systemPackages = [
      pkgs.chromium
      pkgs.curl
      pkgs.xvfb-run
      (pkgs.callPackage ../modules/chrome/session/package.nix { })
      (pkgs.runCommandLocal "chromium-takes-the-open-tabs-extension" { } ''
        mkdir -p $out/share/chromium/extensions
        ln -s /usr/share/google-chrome/extensions/${id}.json $out/share/chromium/extensions/
      '')
    ];
    environment.pathsToLink = [ "/share/chromium" ];
    # Pages of the web for the session case, at an address of their own.
    networking.hosts."127.0.0.1" = [ "pages.example.org" ];
    systemd.services.pages = {
      wantedBy = [ "multi-user.target" ];
      serviceConfig.ExecStart = "${pkgs.python3}/bin/python3 -m http.server 80 --bind 127.0.0.1 --directory ${pages}";
    };
    virtualisation.memorySize = 2048;
  };

  tests = ./chrome-extension-vm-test.py;
  inherit values;
}
