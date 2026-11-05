# swayimg, for a quick look at a picture: a screenshot, a download, a
# photo, a drawing. It's light and quick to open, and it's only a look:
# editing is GIMP's and Inkscape's, a key away.
#
# Its settings are a Lua script, init.lua, which stays Lua. The theme reaches
# it through nix.lua beside it, written here: the theme's colours as the
# numbers swayimg takes, its monospaced face and sizes, and the programs
# the keys run. Behind a picture is the deepest colour, so the picture is
# what's seen, with a quiet checkerboard under anything transparent. The name
# and its place in the folder show over it in the monospaced face, as a
# name and its figures are everywhere, for two
# seconds as it opens, so nothing sits on the picture for long, and the
# gallery marks the one selected with the accent's hairline, lifted a
# little so it's found on a busy picture.
#
# It opens floating in the middle of the screen, a look rather than a place
# to work, and ZZ or Space w puts it away, as they close everything of
# Joel's; q and Escape close nothing. Its keys are Vim's, as Joel's Vim has
# them, each with one meaning, and each arrow does what its letter does:
# h and l go to the previous and next picture, j and k move one larger
# than the window, and in the gallery h j k l choose that way; gg and G,
# and Home and End, go to the first and last, a number before G to that
# picture and before a move that many; Enter goes between the picture and
# the gallery of its folder; e edits in GIMP, or Inkscape for a drawing,
# and closes the viewer; dd and Delete move the picture to the trash
# rather than deleting it, and u brings it back; yy copies the picture as
# shown; ? and Space ? open the launcher's Shortcuts at its keys; and :
# opens the launcher's command line over the picture, for :d, :y, :open
# and a program, :rotate, a number for that picture, and :q, which
# swayimg-command (command.sh) carries back to it. A number or a key
# waiting for its second waits 300 ms at most, his Neovim's timeoutlen,
# and shows nothing, as his Neovim has showcmd off; [ and ] wait only for
# the b of ]b and [b, the next and previous picture, as Vim's go between
# buffers, and alone do nothing, since turning is :rotate. As a picture
# opens, a quiet line under it names the letters that move on, for as long
# as its name shows.
#
# The keys are the picture viewer's layer of jitsusama.keys, so the
# launcher lists them over its window, and init.lua binds them from the
# same list, handed over in nix.lua.
#
# The launcher finds the screenshots folder's gallery by typing, and holds
# it among the picture viewer's own acts (Screenshots.qml). Its Recent
# there is led by the picture each swayimg shows now, which init.lua keeps
# and Return goes to.
{ niri, quickshell }:
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
    shape
    ;

  # A colour as swayimg's number, 0xAARRGGBB, written as a number rather
  # than a string, opaque unless the theme or the caller gives it an alpha.
  # Where each swayimg keeps the picture it shows now (init.lua).
  viewing = "${config.xdg.stateHome}/swayimg/viewing";

  argb =
    alpha: colour:
    let
      hex = lib.removePrefix "#" colour;
      own = builtins.substring 6 2 hex;
    in
    lib.generators.mkLuaInline "0x${if own == "" then alpha else own}${builtins.substring 0 6 hex}";

  opaque = argb "ff";

  # Points as Linux counts them, 96 to the inch, to the pixels swayimg takes.
  pixels = points: builtins.floor (builtins.fromJSON points * 4 / 3 + 0.5);

  lua = lib.generators.toLua { };

  # Where in swayimg a key works, when not everywhere.
  places = {
    viewer = "on a picture";
    gallery = "in the gallery";
  };

  # Every key worth knowing, as swayimg's keys spell it, with what it does
  # and where. A key with an act is bound by init.lua to that act; one
  # without is swayimg's own, listed as swayimg binds it. The launcher's
  # Shortcuts groups those that go between pictures and those that change
  # how the one in view is shown; the rest are few enough to stay loose.
  #
  # Neovim's keys are here too, as docs/design.md's Vi's Keys has them, each
  # beside the arrow or key it twins, by the same words and the same act, so
  # one row lists them both: h j k l as the arrows, gg and G as Home and
  # End, dd as Delete. A key pressed after a number does it that many times,
  # and G after one goes to that picture; init.lua gathers the number and
  # waits for a pair's second key.
  grouped = group: map (key: key // { inherit group; });
  # A key doing one thing where it works, by its act, with the key it twins.
  twins =
    when: does: act: chords:
    map (chord: {
      inherit
        chord
        when
        does
        act
        ;
    }) chords;
  keys =
    grouped "moving" (
      # [b and ]b, as Vim's go between buffers.
      twins places.viewer "Previous picture" "left" [
        "Left"
        "h"
        "bracketleft b"
      ]
      ++ twins places.viewer "Next picture" "right" [
        "Right"
        "l"
        "bracketright b"
      ]
      ++ twins places.viewer "Up a little, on a picture larger than the window" "up" [
        "Up"
        "k"
      ]
      ++ twins places.viewer "Down a little, on a picture larger than the window" "down" [
        "Down"
        "j"
      ]
      ++ twins places.viewer "First picture" "first" [
        "Home"
        "g g"
      ]
      ++ twins places.viewer "Last picture" "last" [
        "End"
        "Shift+g"
      ]
      ++ [
        # Any number before G, five drawn for them all; G's own act counts.
        {
          chord = "5 Shift+g";
          does = "That picture, by its number";
          when = places.viewer;
        }
        {
          chord = "Return";
          does = "Every picture";
          when = places.viewer;
        }
      ]
      ++ twins places.gallery "Choose a picture that way" "left" [
        "Left"
        "h"
      ]
      ++ twins places.gallery "Choose a picture that way" "right" [
        "Right"
        "l"
      ]
      ++ twins places.gallery "Choose a picture that way" "up" [
        "Up"
        "k"
      ]
      ++ twins places.gallery "Choose a picture that way" "down" [
        "Down"
        "j"
      ]
      ++ twins places.gallery "Previous picture" "left" [ "bracketleft b" ]
      ++ twins places.gallery "Next picture" "right" [ "bracketright b" ]
      ++ twins places.gallery "First picture" "first" [
        "Home"
        "g g"
      ]
      ++ twins places.gallery "Last picture" "last" [
        "End"
        "Shift+g"
      ]
      ++ [
        {
          chord = "5 Shift+g";
          does = "That picture, by its number";
          when = places.gallery;
        }
        {
          chord = "Return";
          does = "Open the picture";
          when = places.gallery;
        }
      ]
    )
    ++ grouped "picture" [
      {
        chord = "Shift+plus";
        does = "Closer";
        when = places.viewer;
      }
      {
        chord = "equal";
        does = "Closer";
        when = places.viewer;
      }
      {
        chord = "minus";
        does = "Further";
        when = places.viewer;
      }
      {
        chord = "f";
        does = "Full screen";
        when = places.viewer;
      }
      {
        chord = "Shift+plus";
        does = "Larger";
        when = places.gallery;
      }
      {
        chord = "equal";
        does = "Larger";
        when = places.gallery;
      }
      {
        chord = "minus";
        does = "Smaller";
        when = places.gallery;
      }
    ]
    ++ [
      {
        chord = "e";
        does = "Edit in GIMP, or Inkscape for a drawing";
        act = "edit";
      }
    ]
    # y alone waits for what it's to copy, as in Vim, and does nothing.
    ++ twins places.viewer "Copy the picture as shown" "copy" [ "y y" ]
    ++ twins places.gallery "Copy the file" "copy" [ "y y" ]
    ++ twins "" "Move to the trash" "trash" [
      "Delete"
      "d d"
    ]
    ++ [
      {
        chord = "u";
        does = "Bring back the last one trashed";
        act = "restore";
      }
      # Escape stops a number or a key waiting for its second, and leaves
      # full screen, and closes nothing; q closes nothing either, and says
      # what does (init.lua), as in every program of Joel's.
      {
        chord = "Escape";
        does = "The picture out of full screen";
        act = "escape";
      }
      {
        chord = "Shift+z Shift+z";
        does = "Close";
        act = "close";
      }
      {
        chord = "space w";
        does = "Close";
        act = "close";
      }
      {
        chord = "Shift+colon";
        does = "A command for the picture";
        act = "command_line";
      }
      # Space ?, as in Joel's Neovim, which shows them everywhere.
      {
        chord = "Shift+slash";
        does = "These keys";
        act = "keys";
      }
      {
        chord = "space Shift+slash";
        does = "These keys";
        act = "keys";
      }
    ];

  # How a key is drawn, as the desktop draws it: a shifted key as what it
  # types. A letter is drawn as it's typed, as Neovim's own lists draw
  # them, since G and g are two keys here, and keys pressed one after
  # another run together, as gg. swayimg draws its words in the one face,
  # with no other to fall back on, and the monospaced face has every
  # arrow the desktop draws, Return's among them.
  faces = config.jitsusama.theme.keys;
  themed = {
    bracketleft = "BracketLeft";
    bracketright = "BracketRight";
    equal = "Equal";
    minus = "Minus";
  };
  typed = {
    "Shift+slash" = "?";
    "Shift+colon" = ":";
    "Shift+plus" = "+";
    space = "Space";
  };
  drawnOne =
    chord:
    let
      shifted = lib.removePrefix "Shift+" chord;
    in
    typed.${chord} or faces.${themed.${chord} or chord}.text or (
      if lib.stringLength chord == 1 then
        chord
      else if lib.stringLength shifted == 1 then
        lib.toUpper shifted
      else
        chord
    );
  # Letters pressed one after another run together, gg, and a named key
  # stands apart, Space ?.
  drawn =
    chord:
    let
      parts = map drawnOne (lib.splitString " " chord);
    in
    lib.concatStringsSep (if lib.all (part: lib.stringLength part == 1) parts then "" else " ") parts;

  given = {
    colour = {
      deepest = opaque colours.deepest;
      sunken = opaque colours.sunken;
      raised = opaque colours.raised;
      text = opaque colours.text;
      strong = opaque colours.strong;
      muted = opaque colours.muted;
      accent = opaque colours.accent;
      # Under the name, so it reads on any picture.
      shadow = argb "d0" colours.shadow;
    };
    mono = font.mono.family;
    size = pixels type.mono.large;
    space = {
      inherit (space.rung)
        tie
        mark
        apart
        inset
        screen
        ;
      # The chessboard behind a picture's clear parts: a texture, not a
      # space between two things, so it's sized by the unit, not a rung.
      square = 2 * space.unit;
    };
    inherit (shape) border;
    inherit places;
    keys = map (key: key // { face = drawn key.chord; }) keys;
    # A gallery cell, about ten across a window the width of the screen.
    thumbnail = 160;
    inherit viewing;
    wl_copy = lib.getExe' pkgs.wl-clipboard "wl-copy";
    gio = lib.getExe' pkgs.glib "gio";
    gimp = {
      name = "GIMP";
      command = "gimp";
    };
    inkscape = {
      name = "Inkscape";
      command = "inkscape";
    };
  };
in
{
  imports = [
    ../quickshell/recents.nix
    ../quickshell/command-lines.nix
    ../theme/home.nix
    niri
    quickshell
  ];

  # The picture each swayimg shows now, for its Recent in the launcher to
  # lead with, kept as mpv keeps what it plays (../mpv/home.nix says why
  # in the state home); its other pictures stay the desktop's list of
  # files opened lately.
  systemd.user.tmpfiles.rules = [ "d ${viewing} 0700 - - -" ];
  # : opens the launcher's command line over the picture, with these
  # commands, each by the key language's name (../keys/language.toml).
  jitsusama.quickshell.launcher.commandLines.swayimg = {
    commands = [
      {
        command = "delete";
        title = "Move to the trash";
        key = "dd";
      }
      {
        command = "yank";
        title = "Copy the picture as shown";
        key = "yy";
      }
      {
        command = "open";
        title = "Open it in another program, by its command";
      }
      {
        command = "rotate";
        title = "Turn it, by 90, 180 or 270 degrees";
      }
      {
        command = "quit";
        title = "Close";
        key = "ZZ";
      }
    ];
    goes = "That picture, by its number";
    run = [ (lib.getExe (pkgs.callPackage ./command.nix { })) ];
  };

  jitsusama.quickshell.launcher.recents = [
    {
      app = "swayimg";
      kind = "file";
      opened = import ../quickshell/kept-in.nix { inherit lib pkgs; } viewing;
    }
  ];

  programs.swayimg = {
    enable = true;
    # Home and End reach the first and last picture even when the folder's
    # first or last file isn't one; see the patch's own words.
    package = pkgs.swayimg.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [ ./ends-walk-inwards.patch ];
    });
    initLua = ./init.lua;
  };

  xdg.configFile."swayimg/nix.lua".text = ''
    -- Written by modules/swayimg/home.nix from jitsusama.theme.
    return ${lua given}
  '';

  # After niri's own rules, which match no window of swayimg's. It floats
  # over the work, centred, and init.lua sizes it to stand the screen rung
  # from every edge, as every floating frame does.
  xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
    window-rule {
        match app-id="^swayimg$"
        open-floating true
    }
  '';

  jitsusama.keys.swayimg = map (key: removeAttrs key [ "act" ]) keys;

  # After GIMP's, since it's about the same pictures.
  jitsusama.quickshell.launcher.providers.swayimg = {
    order = 47;
    source = ./Screenshots.qml;
    folder = "${config.home.homeDirectory}/Pictures/Screenshots";
  };
}
