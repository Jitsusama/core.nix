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
    default = 600;
    description = ''
      How many seconds with nothing typed or moved before the cat curls up
      asleep: ten minutes, as long as the shell waits to count Joel away,
      since it grows livelier the less he's engaged ("mild movements when
      i'm active, increeeasing as I'm less engaged", 2026-11-12) and sleeps
      only once he's gone.
    '';
  };

  config = {
    jitsusama.quickshell.surfaces.MarginCat = ./MarginCat.qml;
    xdg.configFile."quickshell/surfaces/MarginCat.js".source = ./MarginCat.js;
    # Its settings in its own file beside it, read as it starts.
    xdg.configFile."quickshell/surfaces/MarginCat.json".text = builtins.toJSON {
      inherit (config.jitsusama.quickshell.cat) dozes;
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
