# The margin's cat: the empty places' cat, sitting in the gap above the
# column Joel is in (MarginCat.qml), a surface of the shell's added from here
# so a machine has it by importing this module, and leaves it out by not.
{ niri, quickshell }:
{ config, lib, ... }:
{
  imports = [
    niri
    quickshell
  ];

  options.jitsusama.quickshell.cat.dozes = lib.mkOption {
    type = lib.types.ints.positive;
    default = 180;
    description = ''
      How many seconds with nothing typed or moved before the cat curls up
      asleep: three minutes, so it plays livelier for about two once he's
      left the keys ("mild movements when i'm active, increeeasing as I'm
      less engaged", 2026-11-12), then sleeps and draws nothing until a
      key, rather than playing all night for an empty room.
    '';
  };

  options.jitsusama.quickshell.cat.pauses = lib.mkOption {
    type = lib.types.ints.positive;
    default = 10;
    description = ''
      How many seconds with no keys before the cat counts Joel paused and
      its scenes grow a little livelier than the single small moves it
      makes while he types.
    '';
  };

  options.jitsusama.quickshell.cat.idles = lib.mkOption {
    type = lib.types.ints.positive;
    default = 60;
    description = ''
      How many seconds with no keys before the cat counts Joel away from
      the keys, and may hop or wander a few cells and back.
    '';
  };

  options.jitsusama.quickshell.cat.pace = lib.mkOption {
    type = lib.types.numbers.positive;
    default = 1;
    description = ''
      How many times sooner than its own rests (MarginCatPlays.js) each of the
      cat's scenes comes: 45 to 75 s while Joel types, 5 to 12 s once he
      pauses, 3 to 8 s once he's left the keys. A test sets it high to see
      scenes in seconds, or low to see none.
    '';
  };

  config = {
    jitsusama.quickshell.surfaces.MarginCat = ./MarginCat.qml;
    # Each screen's cat, beside it rather than a surface of its own.
    xdg.configFile."quickshell/surfaces/MarginCatScreen.qml".source = ./MarginCatScreen.qml;
    # Quickshell makes a folder's types known only for folders it found
    # by following imports, and a surface is loaded by its path, so
    # MarginCat.qml would find no MarginCatScreen type beside it without
    # a qmldir naming it.
    xdg.configFile."quickshell/surfaces/qmldir".text = ''
      MarginCatScreen 1.0 MarginCatScreen.qml
    '';
    xdg.configFile."quickshell/surfaces/MarginCat.js".source = ./MarginCat.js;
    xdg.configFile."quickshell/surfaces/MarginCatPaths.js".source = ./MarginCatPaths.js;
    xdg.configFile."quickshell/surfaces/MarginCatPlays.js".source = ./MarginCatPlays.js;
    xdg.configFile."quickshell/surfaces/MarginCatRoutes.js".source = ./MarginCatRoutes.js;
    xdg.configFile."quickshell/surfaces/MarginCatTiles.js".source = ./MarginCatTiles.js;
    xdg.configFile."quickshell/surfaces/MarginCatWays.js".source = ./MarginCatWays.js;
    xdg.configFile."quickshell/surfaces/MarginCatFalls.js".source = ./MarginCatFalls.js;
    # Its settings in its own file beside it, read as it starts.
    xdg.configFile."quickshell/surfaces/MarginCat.json".text = builtins.toJSON {
      inherit (config.jitsusama.quickshell.cat)
        dozes
        pauses
        idles
        pace
        ;
    };

    # A shared screen shows what Joel shares, not the company he keeps. The
    # cat leaves while a share is on (MarginCat.qml); this keeps it from the
    # frames before the shell hears the share began.
    xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
      layer-rule {
          match namespace="^cat$"
          block-out-from "screencast"
      }
    '';
  };
}
