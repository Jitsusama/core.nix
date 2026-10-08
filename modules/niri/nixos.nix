# niri, the scrolling, tiling compositor Joel works in, and what it needs from
# the system: its session, its portals, Xwayland for the X11 programs left,
# and the tools its brightness keys call.
{ pkgs, ... }:
{
  programs.niri = {
    enable = true;
    # The GTK file chooser, rather than installing Nautilus to provide one.
    useNautilus = false;
  };

  environment.systemPackages = [
    # niri starts it whenever an X11 program needs a display.
    pkgs.xwayland-satellite
    pkgs.brightnessctl
  ];
}
