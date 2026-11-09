# The checks of zoom, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  # An account with zoom on a machine of the platform given.
  on =
    platform:
    account (bareNixos [
      {
        nixpkgs.hostPlatform = lib.mkForce platform;
        home-manager.sharedModules = [ self.homeModules.zoom ];
      }
    ]);
  arm = on "aarch64-linux";
  x86 = on "x86_64-linux";
  hasClient = home: lib.any (package: lib.getName package == "zoom") home.home.packages;
in
onLinux {
  # Where nixpkgs builds Zoom's client the account has it and no web
  # client; where it doesn't, the web client is a program that opens Zoom's
  # own links, and nothing tries to install the client.
  zoom-is-installed-or-opened-on-the-web =
    holds "zoom-is-installed-or-opened-on-the-web"
      "zoom gives x86_64 Linux something other than Zoom's client, or ARM Linux something other than its web client for Zoom's links."
      (
        hasClient x86
        && !(x86.xdg.desktopEntries ? zoom)
        && !(hasClient arm)
        &&
          arm.xdg.desktopEntries.zoom.mimeType == [
            "x-scheme-handler/zoommtg"
            "x-scheme-handler/zoomus"
          ]
      );

  # Each of Zoom's own links joins its meeting in the web client, with a
  # stand-in for the window (tests/zoom-opens-its-links.sh).
  zoom-opens-its-links = pkgs.runCommandLocal "zoom-opens-its-links" { } ''
    ${lib.getExe pkgs.bash} ${./tests/zoom-opens-its-links.sh} ${./opens.sh}
    touch $out
  '';
}
