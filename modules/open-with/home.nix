# The launcher's side of choosing what to open something with: the portal's
# chooser (nixos.nix) asks here, and OpenWith.qml holds the apps it offers
# until one is picked.
{
  jitsusama.quickshell.launcher.providers.openWith = {
    order = 5;
    source = ./OpenWith.qml;
  };
}
