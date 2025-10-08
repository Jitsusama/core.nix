{
  description = "Opinionated Core Nix Configuration";

  inputs = {
    wallpapers.url = "github:Jitsusama/wallpapers.nix";
  };

  outputs =
    { wallpapers, ... }:
    {
      nix-darwin = ./nix-darwin;
      home-manager = ./home-manager;
      wallpapers = wallpapers;
    };
}

