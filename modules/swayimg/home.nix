# swayimg, for a quick look at a picture: a screenshot, a download, a
# photo, a drawing. It's light and quick to open, and it's only a look:
# editing is GIMP's and Inkscape's, a key away.
#
# Its settings are a Lua script, init.lua, which stays Lua. The theme reaches
# it through nix.lua beside it, written here: the theme's colours as the
# numbers swayimg takes, its proportional face and sizes, and the programs
# the keys run. Behind a picture is the deepest colour, so the picture is
# what's seen, with a quiet checkerboard under anything transparent. The name
# and its place in the folder show over it in the theme's face for two
# seconds as it opens, so nothing sits on the picture for long, and the
# gallery marks the one selected with the accent's hairline, lifted a
# little so it's found on a busy picture.
#
# It opens floating in the middle of the screen, a look rather than a place
# to work, and Escape or q puts it away. Its keys follow the keyboard's one
# language: the arrows go that way, to the previous and next picture, or
# around the gallery; Home and End to the first and last; Enter goes
# between the picture and the gallery of its folder; e edits in GIMP, or
# Inkscape for a drawing, and closes the viewer; Delete moves the picture
# to the trash rather than deleting it, and u brings it back; q closes it,
# as in mpv. Vim's keys twin them, as Joel's Vim has them: h j k l the
# arrows, gg and G the first and last, a number before G that picture and
# before a move that many, dd the trash, yy copies the picture as shown,
# and ? and Space ? open the launcher's Shortcuts at its keys. A number or
# a key waiting for its second waits 300 ms at most, his Neovim's
# timeoutlen, and shows nothing, as his Neovim has showcmd off; [ and ]
# start ]b and [b, the next and previous picture, as Vim's go between
# buffers, and turn the picture when no b follows. As a picture opens, a
# quiet line under it names the keys that move on, for as long as its name
# shows.
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
      # A moment after them with no b, as Vim's timeout, they turn.
      {
        chord = "bracketleft";
        does = "Turn left";
        when = places.viewer;
        act = "turn_left";
      }
      {
        chord = "bracketright";
        does = "Turn right";
        when = places.viewer;
        act = "turn_right";
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
      # full screen, before it closes.
      {
        chord = "Escape";
        does = "Close";
        act = "escape";
      }
      # As in mpv.
      {
        chord = "q";
        does = "Close";
        act = "close";
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
  # with no other to fall back on, and the sans face has no return arrow,
  # so there Return is its name.
  faces = config.jitsusama.theme.keys;
  themed = {
    bracketleft = "BracketLeft";
    bracketright = "BracketRight";
    equal = "Equal";
    minus = "Minus";
  };
  typed = {
    "Shift+slash" = "?";
    "Shift+plus" = "+";
    Return = "Enter";
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
    sans = font.sans.family;
    size = pixels type.sans.large;
    space = {
      inherit (space) s l;
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
    ../theme/home.nix
    niri
    quickshell
  ];

  # The picture each swayimg shows now, for its Recent in the launcher to
  # lead with, kept as mpv keeps what it plays (../mpv/home.nix says why
  # in the state home); its other pictures stay the desktop's list of
  # files opened lately.
  systemd.user.tmpfiles.rules = [ "d ${viewing} 0700 - - -" ];
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

  # After niri's own rules, which match no window of swayimg's. It opens at
  # four fifths of the screen, centred, with the desktop showing around it:
  # swayimg's own size is a fixed 1280 by 720, too wide for a laptop's
  # screen at twice its scale.
  xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
    window-rule {
        match app-id="^swayimg$"
        open-floating true
        default-column-width { proportion 0.8; }
        default-window-height { proportion 0.8; }
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
