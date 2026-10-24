# The checks of qt, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  ...
}:
{
  # Qt programs take GTK's look from niri's environment, and Quickshell, which
  # draws from the theme itself, is started without it.
  qt-programs-wear-the-theme =
    let
      home = account graphical;
      service = home.systemd.user.services.quickshell.Service;
    in
    holds "qt-programs-wear-the-theme"
      "Qt programs don't get GTK's platform theme, or Quickshell gets it too."
      (
        home.systemd.user.sessionVariables.QT_QPA_PLATFORMTHEME or null == "gtk3"
        && lib.elem "QT_QPA_PLATFORMTHEME" (service.UnsetEnvironment or [ ])
      );
}
