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
#
# A page speaks Vim's keys through Vimium, the one such extension the
# browser's managers let it take, whose settings Chrome syncs and no policy
# sets. So they're written to ~/.config/jitsusama/vimium.json, the file its
# options page restores a backup from, once, by hand: its own keys with the
# key language's beside them (../keys/language.toml), as j scrolls, gg goes
# to the top, Ctrl+o goes back and ZZ closes the tab. Its own H, L and x
# stay, and f and F show the links' hints; Mail and GitHub, with keys of
# their own, are left to them (vimium.json).
{
  config,
  lib,
  osConfig ? { },
  pkgs,
  ...
}:
let
  grouped = group: map (key: key // { inherit group; });

  # Vimium's command for each of the language's acts it has, with what it
  # does in the launcher's words and the group it's listed in there.
  vimium = group: command: does: { inherit group command does; };
  vimiumActs = {
    down = vimium "scrolling" "scrollDown" "Scroll down";
    up = vimium "scrolling" "scrollUp" "Scroll up";
    left = vimium "scrolling" "scrollLeft" "Scroll left";
    right = vimium "scrolling" "scrollRight" "Scroll right";
    scroll-down = vimium "scrolling" "scrollDown" "Scroll down a line";
    scroll-up = vimium "scrolling" "scrollUp" "Scroll up a line";
    half-down = vimium "scrolling" "scrollPageDown" "Down half a screen";
    half-up = vimium "scrolling" "scrollPageUp" "Up half a screen";
    screen-down = vimium "scrolling" "scrollFullPageDown" "Down a screen";
    screen-up = vimium "scrolling" "scrollFullPageUp" "Up a screen";
    first = vimium "scrolling" "scrollToTop" "The top of the page";
    last = vimium "scrolling" "scrollToBottom" "The bottom of the page";
    jump-back = vimium "pages" "goBack" "Back a page";
    jump-forward = vimium "pages" "goForward" "Forward a page";
    search-forward = vimium "finding" "enterFindMode" "Find words on the page";
    next = vimium "finding" "performFind" "The next words found";
    previous = vimium "finding" "performBackwardsFind" "The words found before";
    copy = vimium "links" "copyCurrentUrl" "Copy the page's address";
    close = vimium "tabs" "removeTab" "Close the tab";
  };

  # Each act's keys Vimium reads: letters and Ctrl's. Not a named key, as
  # Page Down, which Chrome scrolls by itself, nor Space, which scrolls the
  # page, so Space w stays out and ZZ closes.
  vimiumKeys =
    act:
    lib.filter (
      key: builtins.match "<C-.>|[^<]+" key != null
    ) config.jitsusama.keyLanguage.acts.${act}.keys;

  # Every act's keys, each with its Vimium command and words.
  vimiumBinds = lib.concatMap (
    act: map (key: vimiumActs.${act} // { inherit act key; }) (vimiumKeys act)
  ) (lib.attrNames vimiumActs);

  # The lines Vimium reads, its modifiers lower case, as it writes them.
  keyMappings = lib.concatMapStrings (
    bind: "map ${lib.replaceStrings [ "<C-" ] [ "<c-" ] bind.key} ${bind.command}\n"
  ) vimiumBinds;

  # A key in the launcher's spelling: <C-d> is Ctrl+d, G is Shift+g and gg
  # is g g.
  chordOf =
    key:
    lib.concatMapStringsSep " " (
      token:
      let
        ctrl = builtins.match "<C-(.)>" token;
      in
      if ctrl != null then
        "Ctrl+${lib.head ctrl}"
      else if lib.toLower token != token then
        "Shift+${lib.toLower token}"
      else
        token
    ) (map lib.head (lib.filter lib.isList (builtins.split "(<C-.>|.)" key)));

  # On a page with Vimium on, as any but Mail's and GitHub's.
  withVimium = "on a page, with Vimium on";

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
        when = "in a field, or where Vimium is off";
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
    ]
    # Vimium's, the language's and its own, each in the group of the
    # browser's own like it.
    ++ map (key: key // { when = withVimium; }) (
      map (bind: {
        chord = chordOf bind.key;
        inherit (bind) act does group;
      }) vimiumBinds
      ++ [
        {
          chord = "f";
          does = "Show the links' hints, to follow one";
          group = "links";
        }
        {
          chord = "Shift+f";
          does = "Show the links' hints, to open one in a new tab";
          group = "links";
        }
        {
          chord = "Shift+h";
          does = "Back a page";
          group = "pages";
        }
        {
          chord = "Shift+l";
          does = "Forward a page";
          group = "pages";
        }
        {
          chord = "x";
          does = "Close the tab";
          group = "tabs";
        }
      ]
    );
  # What reads Chrome's sites, its downloads and its session (recent-sites).
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

  # Its sites, and its tabs open now. Where the machine installs the
  # extension, they're the host's, which Go To brings forward and Close It
  # closes by the ids it gave; otherwise, and whenever the host lists
  # nothing, as with a managed Chrome refusing the extension, they're its
  # own session file's, which needs no extension but can't name a tab to
  # the host, so recent-sites gives those no id and Go To brings the
  # tab's window forward alone. Without the host there's no Close It.
  jitsusama.quickshell.launcher.recents = [
    (
      {
        app = "google-chrome";
        kind = "site";
        asked = true;
        list = [ recentSites ];
        dated = true;
        opened = [
          recentSites
          "open"
        ]
        ++ lib.optionals (openTabs != null) [
          "--tabs-from"
          openTabs
        ];
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

  # Vimium's settings, for its options page to restore once, as the comment
  # at the top says; Chrome syncs them from there.
  xdg.configFile."jitsusama/vimium.json".text = builtins.toJSON (
    lib.importJSON ./vimium.json // { inherit keyMappings; }
  );

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
