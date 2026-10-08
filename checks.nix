# Everything `nix flake check` runs.
#
# Each module and role is added, on its own, to a bare machine of every class
# it can run on: NixOS and nix-darwin for system modules, and an account on
# either for home-manager ones. A module that quietly depends on another fails
# here rather than on a machine. Then come the promises roles make when they
# are combined, and the example machines, which use core only through its
# outputs, as a machine repository does. Evaluating catches broken options and
# failed assertions without building anything, which keeps the checks fast.
{
  self,
  pkgs,
  nixpkgs,
  nix-darwin,
  formatting,
}:
let
  inherit (pkgs) lib;

  # The least a machine needs to evaluate, plus home-manager and nixpkgs'
  # settings, which every machine has, and one account for home-manager to
  # configure, so that the home half of a role or module is evaluated too.
  bareNixos =
    modules:
    nixpkgs.lib.nixosSystem {
      modules = [
        self.nixosModules.home-manager
        self.nixosModules.nixpkgs
        {
          nixpkgs.hostPlatform = "x86_64-linux";
          boot.loader.grub.enable = false;
          fileSystems."/" = {
            device = "none";
            fsType = "tmpfs";
          };
          users.users.someone.isNormalUser = true;
          home-manager.users.someone.home.stateVersion = "26.05";
          system.stateVersion = "26.05";
        }
      ]
      ++ modules;
    };

  bareDarwin =
    modules:
    nix-darwin.lib.darwinSystem {
      modules = [
        self.darwinModules.home-manager
        self.darwinModules.nixpkgs
        {
          nixpkgs.hostPlatform = "aarch64-darwin";
          system.primaryUser = "someone";
          users.users.someone.home = "/Users/someone";
          home-manager.users.someone.home.stateVersion = "26.05";
          system.stateVersion = 6;
        }
      ]
      ++ modules;
    };

  # A home-manager module, given to the bare machine's one account.
  inAccount = module: { home-manager.users.someone.imports = [ module ]; };

  # A check that passes when the machine evaluates. Writing out the path of
  # its derivation forces the whole configuration while building none of it.
  evaluates =
    name: machine:
    pkgs.writeText name (
      builtins.unsafeDiscardStringContext machine.config.system.build.toplevel.drvPath
    );

  # A check that passes when two machines evaluate to the same derivation.
  same =
    name: message: one: other:
    if one.config.system.build.toplevel.drvPath == other.config.system.build.toplevel.drvPath then
      evaluates name one
    else
      failure name message;

  # A check that passes when a condition about a machine holds.
  holds =
    name: message: condition:
    if condition then pkgs.writeText name "" else failure name message;

  failure =
    name: message:
    pkgs.runCommandLocal name { } ''
      echo ${lib.escapeShellArg message}
      exit 1
    '';

  account = machine: machine.config.home-manager.users.someone;

  # One check per module, named after where it was added: "darwin-zsh" is the
  # zsh module on a bare Mac.
  checkEach =
    where: addToBareMachine: modules:
    lib.mapAttrs' (name: module: {
      name = "${where}-${name}";
      value = evaluates "${where}-${name}" (addToBareMachine module);
    }) modules;

  # The files a machine imports, and the examples that show how. Documentation
  # may name an employer; these may not.
  imported = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./modules
      ./roles
      ./examples
    ];
  };
in
checkEach "nixos" (module: bareNixos [ module ]) self.nixosModules
// checkEach "darwin" (module: bareDarwin [ module ]) self.darwinModules
// checkEach "home-on-nixos" (module: bareNixos [ (inAccount module) ]) self.homeModules
// checkEach "home-on-darwin" (module: bareDarwin [ (inAccount module) ]) self.homeModules
// {
  inherit formatting;

  # Roles import the roles beneath them, so a machine importing two roles
  # imports the shared ones twice. That has to count once.
  nixos-role-twice-is-once =
    same "nixos-role-twice-is-once" "Importing a NixOS role twice changed the machine."
      (bareNixos [ self.nixosModules.workstation ])
      (bareNixos [
        self.nixosModules.workstation
        self.nixosModules.workstation
      ]);
  darwin-role-twice-is-once =
    same "darwin-role-twice-is-once" "Importing a nix-darwin role twice changed the machine."
      (bareDarwin [ self.darwinModules.workstation ])
      (bareDarwin [
        self.darwinModules.workstation
        self.darwinModules.workstation
      ]);

  # A machine can remove any module a role brought in, including the ones
  # made by giving a file one of core's inputs.
  disabling-a-module-removes-it =
    let
      workstationWithoutNeovim = bareNixos [
        self.nixosModules.workstation
        { home-manager.users.someone.disabledModules = [ self.homeModules.neovim ]; }
      ];
    in
    holds "disabling-a-module-removes-it" "disabledModules left Neovim on the workstation." (
      !(account workstationWithoutNeovim).programs.neovim.enable
    );

  example-nixos = evaluates "example-nixos" (
    import ./examples/nixos.nix {
      core = self;
      inherit nixpkgs;
    }
  );
  example-darwin = evaluates "example-darwin" (
    import ./examples/darwin.nix {
      core = self;
      inherit nix-darwin;
    }
  );

  # core.nix is public and shared by personal and work machines alike, so
  # anything tied to an employer belongs in the work repository instead.
  nothing-work-specific =
    pkgs.runCommandLocal "nothing-work-specific" { nativeBuildInputs = [ pkgs.ripgrep ]; }
      ''
        cd ${imported}
        if rg --ignore-case --fixed-strings --line-number --no-heading \
          -e shopify -e /opt/dev -e gitstream -e devx .; then
          echo "core.nix is shared by every machine; the lines above belong in the work repository."
          exit 1
        fi
        touch $out
      '';
}
