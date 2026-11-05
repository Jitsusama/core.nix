# zathura, for reading a document: a PDF, an EPUB, an XPS, a DjVu scan or
# PostScript. It's light, quick and kept to the pages: no bars at rest, the
# command line only while one is typed, so a document opened to glance at
# shows nothing but itself.
#
# Every colour it draws is from the theme: the desk behind the pages is the
# deepest colour, and its bar, command line, contents and notices are drawn
# as the rest of the desktop's are, in the monospaced face, since it's a
# command line and page numbers. The pages
# themselves are recoloured into the theme, paper to the background and ink
# to the text, keeping each colour's hue so a chart still reads, and leaving
# photographs as they are; Ctrl+R shows a page as printed and back.
#
# Search hits are washed in a light neutral and the current one in the
# accent, the screen's one focal point, so the words stay legible under
# both. Text selected goes to the
# clipboard, where a paste looks for it, rather than only the primary
# selection. zathura remembers where each document was left and opens it
# there again. Each document it's started with is said to the desktop's
# list of files opened lately (../open-with/says-what-it-opens.nix), so its
# Recent in the launcher lists them, led by the document each zathura has
# open now, which zathura says over its bus and open-now reads, and which
# Return goes to.
#
# Its keys are Vim's, which are zathura's own, as his Vim has them, each
# with one meaning, and each arrow does what its letter does, unlisted: j and k
# scroll down and up, h and l sideways, J and K turn the pages as Page Down
# and Page Up do, gg and G go to the first and the last as Home and End do,
# a count before G to that page, Ctrl+D and Ctrl+U half a screen, Ctrl+F
# and Ctrl+B a screen, / and ? find on and back and n and N again, m and '
# mark a place and go back to it, and Ctrl+O and Ctrl+I go back and on
# along the jumps. ]b and [b open the next document in its folder and the
# one before, by name, as Vim's go between buffers (next-document/). Tab
# goes back to the place used last (a link followed, a search, the first
# page) and again returns, c opens the contents, Space ? shows its keys in
# the launcher, over its window, as in Joel's Neovim, and : is zathura's
# own command line, which is Vim's already. ZZ and Space w close it, as
# they close everything of his, and q closes nothing. Space scrolls no
# more, since it leads, which a patch lets it.
#
# A key waiting for its second isn't shown, as Joel's Neovim has showcmd
# off, and zathura's bar, which would show it, stays hidden.
#
# Comic book archives are left out: that plugin also claims folders and
# zip, tar and 7z archives, which would open in zathura.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme)
    colours
    font
    type
    space
    ;
  rgba = import ../theme/rgba.nix { inherit lib; };
  says = pkgs.callPackage ../open-with/says-what-it-opens.nix { };
  open-now = lib.getExe' (pkgs.callPackage ../open-with/recent-files.nix { }) "open-now";
  plugins = pkgs.zathuraPkgs;

  # zathura's jumplist goes back and forward, but only Ctrl+O and Ctrl+I
  # each go one way, so a patch lets one key toggle between the last two.
  # Space never joins what zathura waits on, so a patch lets it lead a
  # buffered command, Space ?. A number before a key that runs a command
  # ran it once, so a patch has it run that many times, as 3]b does.
  core = plugins.zathura_core.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [
      ./exec-repeats.patch
      ./jumplist-toggles.patch
      ./space-leads.patch
    ];
  });
  next-document = lib.getExe (pkgs.callPackage ./next-document/package.nix { });

  # Every key worth knowing: the chord as niri spells it, what it
  # does, and, where it isn't zathura's own already, what zathura is told to
  # do on it (`run`), on its key as zathura spells one (`key`). The
  # launcher lists them over zathura's window: those that go page by page
  # or to the document beside as Turning pages, those that move within a
  # page as Scrolling, those that size the page as Fitting the page, those that go back to a place as Places, the searches as Finding
  # words, and the rest loose. Keys that do one thing say it in the same
  # words, so the launcher draws them as one row. An arrow does what its
  # letter does but is never listed. keys-test.sh presses each.
  grouped = group: map (key: key // { inherit group; });
  keys =
    grouped "turning" [
      {
        chord = "Page_Up";
        does = "The previous page";
      }
      {
        chord = "Shift+k";
        does = "The previous page";
      }
      {
        chord = "Page_Down";
        does = "The next page";
      }
      {
        chord = "Shift+j";
        does = "The next page";
      }
      {
        chord = "Home";
        does = "The first page";
      }
      {
        chord = "g g";
        does = "The first page";
      }
      {
        chord = "End";
        does = "The last page";
      }
      {
        chord = "Shift+g";
        does = "The last page";
      }
      # Any number before G, five drawn for them all.
      {
        chord = "5 Shift+g";
        does = "That page";
      }
      # As Vim's ]b and [b go between buffers, $DBUS the window, whose
      # zathura says what it shows.
      {
        chord = "bracketright b";
        key = "]b";
        run = ''exec "${next-document} next $DBUS"'';
        does = "The next document in its folder";
      }
      {
        chord = "bracketleft b";
        key = "[b";
        run = ''exec "${next-document} previous $DBUS"'';
        does = "The document before in its folder";
      }
    ]
    ++ grouped "scrolling" [
      {
        chord = "k";
        does = "Up a little";
      }
      {
        chord = "j";
        does = "Down a little";
      }
      {
        chord = "h";
        does = "Left a little";
      }
      {
        chord = "l";
        does = "Right a little";
      }
      {
        chord = "Ctrl+u";
        # Not the reader's words for its Ctrl+U, since the launcher lists
        # every program's keys together, one row for each.
        does = "Half a screen up";
      }
      {
        chord = "Ctrl+d";
        does = "Half a screen down";
      }
      {
        chord = "Ctrl+b";
        does = "Up a screen";
      }
      {
        chord = "Ctrl+f";
        does = "Down a screen";
      }
    ]
    ++ grouped "fitting" [
      {
        chord = "Equal";
        key = "=";
        run = "zoom in";
        does = "The pages larger";
      }
      {
        chord = "Minus";
        does = "The pages smaller";
      }
      {
        chord = "a";
        does = "A whole page in view";
      }
      {
        chord = "s";
        does = "The page's width in view";
      }
    ]
    ++ grouped "places" [
      {
        chord = "Tab";
        key = "<Tab>";
        run = "jumplist toggle";
        does = "The place used last";
      }
      {
        chord = "Ctrl+o";
        does = "Back along the jumps";
      }
      {
        chord = "Ctrl+i";
        does = "On along the jumps";
      }
      {
        chord = "m";
        does = "Mark a place, go back to it";
      }
      {
        chord = "'";
        does = "Mark a place, go back to it";
      }
    ]
    ++ grouped "finding" [
      {
        chord = "slash";
        does = "Find words in the document";
      }
      {
        chord = "Shift+slash";
        does = "Find words going back";
      }
      {
        chord = "n";
        does = "The next words found";
      }
      {
        chord = "Shift+n";
        does = "The words found before";
      }
    ]
    ++ [
      {
        chord = "c";
        key = "c";
        run = "toggle_index";
        does = "The contents";
      }
      {
        chord = "Ctrl+r";
        does = "As printed, or in the theme";
      }
      # Space, then ?, as zathura spells a buffered command.
      {
        chord = "space Shift+slash";
        key = ''" ?"'';
        run = ''exec "launcher-say collection shortcuts"'';
        does = "The document's keys";
      }
      # q closes nothing, as in every program of Joel's; ZZ and Space w
      # close, as they close everything of his.
      {
        chord = "Shift+z Shift+z";
        key = "ZZ";
        run = "quit";
        does = "Close the document";
      }
      {
        chord = "space w";
        key = ''" w"'';
        run = "quit";
        does = "Close the document";
      }
    ];
in
{
  imports = [
    ../keys/home.nix
    ../quickshell/recents.nix
    ../theme/home.nix
  ];

  # What each zathura has open now, for its Recent to lead with, read as
  # it's stepped into; its other files stay the desktop's list of files
  # opened lately.
  jitsusama.quickshell.launcher.recents = [
    {
      app = "org.pwmt.zathura";
      kind = "file";
      opened = [
        open-now
        "zathura"
      ];
    }
  ];

  programs.zathura = {
    enable = true;
    package = says {
      package = pkgs.zathura.override {
        zathura_core = core;
        plugins = [
          plugins.zathura_pdf_mupdf
          plugins.zathura_djvu
          plugins.zathura_ps
        ];
      };
      command = "zathura";
    };
    options = {
      font = "${font.mono.family} ${type.mono.body}";
      guioptions = "";

      default-bg = colours.deepest;
      default-fg = colours.text;
      page-v-padding = space.s;
      page-h-padding = space.s;
      render-loading-bg = colours.background;
      render-loading-fg = colours.muted;

      recolor = true;
      recolor-keephue = true;
      recolor-reverse-video = true;
      recolor-lightcolor = colours.background;
      recolor-darkcolor = colours.text;

      statusbar-bg = colours.sunken;
      statusbar-fg = colours.text;
      statusbar-h-padding = space.m;
      statusbar-v-padding = space.s;
      statusbar-home-tilde = true;
      inputbar-bg = colours.sunken;
      inputbar-fg = colours.strong;

      completion-bg = colours.sunken;
      completion-fg = colours.text;
      completion-group-bg = colours.deepest;
      completion-group-fg = colours.muted;
      completion-highlight-bg = colours.selection;
      completion-highlight-fg = colours.strong;

      index-bg = colours.background;
      index-fg = colours.text;
      index-active-bg = colours.selection;
      index-active-fg = colours.strong;

      notification-bg = colours.raised;
      notification-fg = colours.text;
      notification-warning-bg = colours.warning;
      notification-warning-fg = colours.on_accent;
      notification-error-bg = colours.alert;
      notification-error-fg = colours.on_accent;

      # Washed over the words, which are lighter than the page: a wash as dark
      # as the selection would dull them, so a lighter one lifts the page to
      # about the selection's colour and leaves the words legible.
      highlight-color = rgba colours.muted "0.35";
      highlight-active-color = rgba colours.accent "0.4";
      highlight-fg = colours.strong;

      signature-success-color = rgba colours.success "0.9";
      signature-warning-color = rgba colours.warning "0.9";
      signature-error-color = rgba colours.alert "0.9";

      selection-clipboard = "clipboard";
      window-title-basename = true;
    };
    # Space leads rather than scrolls, and Shift+Space with it, and q closes
    # nothing, in any of zathura's modes.
    extraConfig = ''
      unmap <Space>
      unmap <S-Space>
      unmap q
      unmap [fullscreen] q
      unmap [presentation] q
      unmap [index] q
    '';
    # The contents close on the key that opened them, as on Escape.
    mappings =
      lib.listToAttrs (map (key: lib.nameValuePair key.key key.run) (lib.filter (key: key ? run) keys))
      // {
        "[index] c" = "toggle_index";
        # Full screen closes as the window does.
        "[fullscreen] ZZ" = "quit";
      };
  };

  # The launcher lists these over zathura's window, by the same words.
  jitsusama.keys.zathura = map (key: {
    inherit (key) chord does;
    group = key.group or "";
  }) keys;
}
