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
      "gimp"
      "inkscape"
      "raycast"
      "spotify"
    ];
  };
}

