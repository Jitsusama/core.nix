# niri, the scrolling, tiling compositor Joel works in, and what it needs from
# the system: its session, its portals, Wayland for Chromium and Electron
# programs, Xwayland for the X11 programs left, and the tools its brightness
# and media keys call.
{ pkgs, ... }:
{
  programs.niri = {
    enable = true;
    # The GTK file chooser, rather than installing Nautilus to provide one.
    useNautilus = false;
  };

  # nixpkgs's wrappers for Chromium and Electron programs, such as Chrome,
  # Slack and 1Password, ask for Wayland only when this is set, and Electron
  # doesn't ask on its own. Through Xwayland, every monitor would share the
  # smallest one's DPI, and each program would scale itself from it.
  environment.sessionVariables.NIXOS_OZONE_WL = "1";

  environment.systemPackages = [
    # niri starts it whenever an X11 program needs a display.
    pkgs.xwayland-satellite
    pkgs.brightnessctl
    pkgs.playerctl
  ];
}
