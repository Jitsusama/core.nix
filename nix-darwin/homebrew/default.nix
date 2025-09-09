{ ... }:
{
  homebrew = {
    enable = true;
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

