{
  homebrew = {
    enable = true;
    # A rebuild installs what is listed and touches nothing else: it never
    # updates brew, upgrades a formula or removes one it does not know.
    onActivation = {
      autoUpdate = false;
      cleanup = "none";
      upgrade = false;
    };
    casks = [
      "ghostty"
      "gimp"
      "inkscape"
      "raycast"
      "spotify"
    ];
  };
}
