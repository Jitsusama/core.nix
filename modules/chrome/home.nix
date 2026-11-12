# Chrome's keys, for the launcher to list over its windows: the browser's
# own, and those of a window it opens for one site, such as Calendar or
# Mail, whose app id names the site (launcher/KeyLayers.js). Chrome comes
# from wherever the machine gets it, so this lists its keys and installs
# nothing; a machine with Chrome imports it. Chrome takes no settings for
# its keys, so these are its own, which mostly speak the keyboard's
# language already (docs/design.md, The Keyboard): T tabs, W closes, a
# number is that tab, and the arrows go back and forward. Its Recent in the
# launcher lists the sites it visited lately and the files it downloaded,
# each read from its history as Chrome is stepped into (recent-sites), and
# its own acts there open a repository's page on GitHub, the link copied
# or the latest download (ChromeProgram.qml).
{
  config,
  lib,
  osConfig ? { },
  pkgs,
  ...
}:
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
  # What reads Chrome's sites and its downloads (recent-sites).
  recentSites = lib.getExe (pkgs.callPackage ./recent-sites/package.nix { });
  # The extension's host, chrome-open-tabs, where the machine's own Chrome
  # module (nixos.nix) wrote the file Chrome starts it from, built from the
  # same package, so it's the program Chrome runs; null on a machine
  # without that module, whose Chrome has no extension to answer the host,
  # or with home-manager on its own, where nothing says.
  hostFile = "opt/chrome/native-messaging-hosts/jitsusama.chrome_open_tabs.json";
  openTabs =
    if osConfig ? environment.etc.${hostFile} then
      lib.getExe (pkgs.callPackage ./open-tabs/host/package.nix { })
    else
      null;
in
{
  imports = [
    ../keys/home.nix
    ../quickshell/providers.nix
    ../quickshell/recents.nix
  ];

  jitsusama.keys.chrome = keys;

  # Its sites, and its tabs open now, which the programs daemon reads
  # itself, with no process started, as Chrome is stepped into and as
  # Chrome writes its session while it's watched. Where the machine
  # installs the extension, they're its host's, asked on its socket, which
  # Go To brings forward and Close It closes by the ids it gave; otherwise,
  # and whenever the host isn't there or lists nothing, as with a managed
  # Chrome refusing the extension, they're the profile's own session
  # file's (./session), which needs no extension but can't name a tab to
  # the host, so those have no id and Go To brings the tab's window
  # forward alone. Without the host there's no Close It.
  jitsusama.quickshell.launcher.recents = [
    (
      {
        app = "google-chrome";
        kind = "site";
        asked = true;
        list = [ recentSites ];
        dated = true;
        session = {
          profile = "${config.home.homeDirectory}/.config/google-chrome/Default";
          host = openTabs != null;
        };
      }
      // lib.optionalAttrs (openTabs != null) {
        go = [
          openTabs
          "go"
          "{id}"
        ];
        close = [
          openTabs
          "close"
          "{id}"
        ];
      }
    )
    # Its downloads, which the programs daemon reads as Chrome's page
    # watches them, as it's stepped into, so the page starts no process.
    {
      app = "google-chrome";
      name = "downloads";
      asked = true;
      list = [
        recentSites
        "downloads"
      ];
    }
  ];

  xdg.configFile."quickshell/launcher/ChromeDownloads.js".source = ./ChromeDownloads.js;

  # Chrome's own acts in the launcher (ChromeProgram.qml): Open a
  # Repository on GitHub, through the git repositories under ~/src and
  # their remotes, which the programs daemon reads, and Open the Copied
  # Link, each in a new window, and Open the Latest Download, as the
  # desktop opens the file. Its downloads, which the programs daemon has
  # recent-sites list as Chrome is stepped into, are in its Recent too. After every other
  # provider, as it adds no rows of its own.
  jitsusama.quickshell.launcher.providers.chrome = {
    order = 52;
    source = ./ChromeProgram.qml;
    # The programs daemon's key for its downloads, the list above.
    downloads = "listed:google-chrome:downloads";
    folder = "${config.home.homeDirectory}/src";
    command = [
      "google-chrome-stable"
      "--new-window"
    ];
  };
}
