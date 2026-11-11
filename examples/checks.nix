# The checks of the example machines, which use core only through its
# outputs and so evaluate every module there is; checks.nix gathers them with
# every other folder's.
{
  evaluates,
  nix-darwin,
  nixpkgs,
  self,
  ...
}:
{
  example-nixos = evaluates "example-nixos" (
    import ./nixos.nix {
      core = self;
      inherit nixpkgs;
    }
  );
  example-laptop = evaluates "example-laptop" (
    import ./laptop.nix {
      core = self;
      inherit nixpkgs;
    }
  );
  example-apple-laptop = evaluates "example-apple-laptop" (
    import ./apple-laptop.nix {
      core = self;
      inherit nixpkgs;
    }
  );
  example-darwin = evaluates "example-darwin" (
    import ./darwin.nix {
      core = self;
      inherit nix-darwin;
    }
  );
}
