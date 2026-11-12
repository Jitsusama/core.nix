# Chrome's keys, for the launcher to list over its windows: the browser's
# own, and those of a window it opens for one site, such as Calendar or
# Mail, whose app id names the site (launcher/KeyLayers.js). Chrome comes
# from wherever the machine gets it, so this lists its keys and installs
# nothing; a machine with Chrome imports it. Chrome takes no settings for
# its keys, so these are its own, which mostly speak the keyboard's
# language already (docs/design.md, The Keyboard): T tabs, W closes, a
# number is that tab, and the arrows go back and forward. Space and
# Shift+Space move a screen and Home and End to an end of the page, F7 lays
# a caret on it to move through and select by the keys, and Ctrl+Shift+a
# searches the open tabs, all with no extension. Its Recent in the
# launcher lists the sites it visited lately and the files it downloaded,
# each read from its history as Chrome is stepped into (recent-sites), and
# its own acts there open a repository's page on GitHub, the link copied
# or the latest download (ChromeProgram.qml).
#
# A page speaks Vim's keys through Vimium, on a machine that sets
# jitsusama.chrome.vimium, since a managed Chrome may refuse it: an
# organization's access checks can turn away a browser with an extension
# they haven't approved, so a machine whose Chrome is managed leaves it
# off, and its launcher lists no keys it can't use. Only a machine with no
# work Google profile in its Chrome sets it, with Vimium taken from the
# Chrome Web Store by hand, never sideloaded. Vimium's settings
# Chrome syncs and no policy sets, and nothing here installs it. So
# they're written to ~/.config/jitsusama/vimium.json, the file its
# options page restores a backup from, once, by hand: its own keys with the
# key language's beside them (../keys/language.toml), as j scrolls, gg goes
# to the top, Ctrl+o goes back and ZZ closes the tab. Its own H, L and x
# stay, and f and F show the links' hints. GitHub (vimium.json) and each
# of Google's pages with a layer of its own (../google/keys.nix), whose
# keys are their own, are left to them: Vimium is off there, and its keys
# leave theirs, so Mail's j is Mail's.
{
  config,
  lib,
  osConfig ? { },
  pkgs,
  ...
}:
let
  grouped = group: map (key: key // { inherit group; });

  # Google's pages with keys of their own, by the layer that lists them
  # and the addresses Vimium is off at.
  googlePages = {
    google-mail = "https?://mail.google.com/*";
    google-calendar = "https?://calendar.google.com/*";
    google-meet = "https?://meet.google.com/*";
    google-docs = "https?://docs.google.com/document/*";
    google-sheets = "https?://docs.google.com/spreadsheets/*";
    google-slides = "https?://docs.google.com/presentation/*";
    google-drive = "https?://drive.google.com/*";
  };
  exclusionRules = lib.mapAttrsToList (_: pattern: {
    inherit pattern;
    passKeys = "";
  }) googlePages;

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

  # On a page with Vimium on, as any but GitHub's and Google's own.
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
    {
      chord = "Ctrl+Shift+a";
      does = "Search the open tabs";
    }
  ];

  # Chrome's own for moving about a page, which work with or without
  # Vimium, but for in a field, where they type or move the text's caret,
  # and on Google's pages, whose own keys take them, as Sheets' Shift+Space
  # selects the row.
  outsideFields = map (
    key:
    key
    // {
      when = "outside a field";
      leaves = lib.attrNames googlePages;
    }
  );

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
        chord = "F7";
        does = "Move through the page by a caret";
      }
      (
        {
          chord = "Ctrl+f";
          does = "Find words on the page";
        }
        // lib.optionalAttrs config.jitsusama.chrome.vimium {
          when = "in a field, or where Vimium is off";
        }
      )
      # Chrome selects the words Ctrl+f found as Escape closes its bar, and
      # Return on a selected link follows it, so a link is followed by its
      # words with no hints.
      {
        chord = "Escape Return";
        does = "Follow the link found";
        when = "after Ctrl+f finds its words";
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
    ++ grouped "scrolling" (outsideFields [
      {
        chord = "Space";
        does = "Down a screen";
      }
      {
        chord = "Shift+Space";
        does = "Up a screen";
      }
      {
        chord = "Home";
        does = "The top of the page";
      }
      {
        chord = "End";
        does = "The bottom of the page";
      }
    ])
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
    # browser's own like it, where the machine has Vimium.
    ++ lib.optionals config.jitsusama.chrome.vimium (
      map
        (
          key:
          key
          // {
            when = withVimium;
            leaves = lib.attrNames googlePages;
          }
        )
        (
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
        )
    );
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

  options.jitsusama.chrome.vimium = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = ''
      Whether this machine's Chrome takes Vimium, so its settings are
      written for Vimium to restore and the launcher lists its keys. Off
      where an organization manages Chrome and hasn't approved it. On only
      on a machine with no work Google profile in Chrome, and with Vimium
      taken from the Chrome Web Store by hand, never sideloaded, since it
      reads and types into every page.
    '';
  };

  config = {
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
    # forward alone. Without the host there's no Close It. Its bookmarks are
    # read from the same profile's Bookmarks and AccountBookmarks files,
    # whole and never written.
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

    # Vimium's settings, for its options page to restore once, as the comment
    # at the top says; Chrome syncs them from there.
    xdg.configFile."jitsusama/vimium.json" = lib.mkIf config.jitsusama.chrome.vimium {
      text = builtins.toJSON (
        let
          own = lib.importJSON ./vimium.json;
        in
        own
        // {
          inherit keyMappings;
          exclusionRules = exclusionRules ++ own.exclusionRules;
        }
      );
    };

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
    # Chrome's open tabs, each found from home by its title or its site, as
    # the programs daemon serves them from Chrome's session, and its
    # bookmarks, but for a page open now, each opened in Chrome by Return.
    jitsusama.quickshell.launcher.providers.tabs = {
      order = 53;
      source = ./Tabs.qml;
    };
  };
}
