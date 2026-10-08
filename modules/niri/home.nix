# Joel's niri: config.kdl says how it behaves, and theme.kdl, written here from
# jitsusama.theme, how it looks. config.kdl is text a machine can add to, with
# lib.mkAfter, such as a panel's scale; niri lets a later setting override an
# earlier one, so the machine's lines win.
{ config, ... }:
let
  colors = config.jitsusama.theme.colors;
in
{
  imports = [ ../theme/home.nix ];

  xdg.configFile."niri/config.kdl".text = builtins.readFile ./config.kdl;

  xdg.configFile."niri/theme.kdl".text = ''
    layout {
        background-color "${colors.background}"
        border {
            active-color "${colors.accent}"
            // Omarchy's inactive border, which its palette doesn't name.
            inactive-color "#595959aa"
        }
        tab-indicator {
            active-color "${colors.accent}"
            inactive-color "${colors.lighter_background}"
        }
        insert-hint {
            color "${colors.accent}80"
        }
    }
    overview {
        backdrop-color "${colors.darker_background}"
    }
  '';
}
