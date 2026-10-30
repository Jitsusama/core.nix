# Chrome's keys, for the launcher to list over its windows: the browser's
# own, and those of a window it opens for one site, such as Calendar or
# Mail, whose app id names the site (launcher/KeyGroups.js). Chrome comes
# from wherever the machine gets it, so this lists its keys and installs
# nothing; a machine with Chrome imports it. Chrome takes no settings for
# its keys, so these are its own, which mostly speak the keyboard's
# language already (docs/design.md, The Keyboard): T tabs, W closes, a
# number is that tab, and the arrows go back and forward. Its Recent in the
# launcher lists the sites it visited lately, read from its history as
# Chrome is stepped into (recent-sites).
{ lib, pkgs, ... }:
let
  grouped = group: map (key: key // { inherit group; });

  # A window for one site has no tabs, so these work in the browser's own.
  inBrowser = map (key: key // { when = "in the browser"; });

  tabs = inBrowser [
    {
      chord = "Ctrl+t";
      does = "A new tab";
    }
    {
      chord = "Ctrl+w";
      does = "Close the tab";
    }
    {
      chord = "Ctrl+Shift+t";
      does = "Open the tab closed last";
    }
    {
      chord = "Ctrl+Tab";
      does = "The next tab";
    }
    {
      chord = "Ctrl+Shift+Tab";
      does = "The tab before";
    }
  ];

  numbers = inBrowser (
    map (number: {
      chord = "Ctrl+${toString number}";
      does = "Tab number ${toString number}";
    }) (lib.range 1 8)
    ++ [
      {
        chord = "Ctrl+9";
        does = "The last tab";
      }
    ]
  );

  keys =
    grouped "tabs" tabs
    ++ grouped "numbers" numbers
    ++ grouped "pages" [
      {
        chord = "Alt+Left";
        does = "Back a page";
      }
      {
        chord = "Alt+Right";
        does = "Forward a page";
      }
      {
        chord = "Ctrl+r";
        does = "Load the page again";
      }
      {
        chord = "Ctrl+l";
        does = "Type an address";
      }
      {
        chord = "Ctrl+f";
        does = "Find words on the page";
      }
      {
        chord = "Ctrl+Plus";
        does = "The page larger";
      }
      {
        chord = "Ctrl+Minus";
        does = "The page smaller";
      }
      {
        chord = "Ctrl+0";
        does = "The page at its own size";
      }
    ]
    ++ [
      {
        chord = "Ctrl+n";
        does = "A new browser window";
      }
      {
        chord = "Ctrl+Shift+n";
        does = "A private browser window";
      }
      {
        chord = "Ctrl+h";
        does = "The pages seen before";
      }
      {
        chord = "Ctrl+j";
        does = "The downloads";
      }
      {
        chord = "F11";
        does = "The page on the whole screen";
      }
    ];
  # What reads Chrome's sites and its session (recent-sites).
  recentSites = lib.getExe (pkgs.callPackage ./recent-sites/package.nix { });
in
{
  imports = [
    ../keys/home.nix
    ../quickshell/recents.nix
  ];

  jitsusama.keys.chrome = keys;

  # Its sites, and its tabs open now as its own session file holds them,
  # which needs no extension, so a browser that allows none still says
  # them. The file is only read, so Go To brings a tab's window forward
  # rather than the tab, and nothing closes one; the extension's host
  # (open-tabs) could, where an extension is allowed.
  jitsusama.quickshell.launcher.recents = [
    {
      app = "google-chrome";
      kind = "site";
      asked = true;
      list = [ recentSites ];
      opened = [
        recentSites
        "open"
      ];
    }
  ];
}
