{
  description = "Opinionated Core Nix Configuration";

  inputs = {
    wallpapers.url = "github:Jitsusama/wallpapers.nix";
    neovim-pi = {
      url = "github:Jitsusama/neovim.pi";
      flake = false;
    };
  };

  outputs =
    { wallpapers, neovim-pi, ... }:
    {
      nix-darwin = ./nix-darwin;
      home-manager = ./home-manager;
      wallpapers = wallpapers;
      neovim-pi = neovim-pi;
    };
}
