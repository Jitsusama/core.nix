{
  description = "Opinionated Core Nix Configuration";

  outputs =
    { ... }:
    {
      nix-darwin = ./nix-darwin;
      home-manager = ./home-manager;
    };
}

