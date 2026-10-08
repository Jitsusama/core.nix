# The index of core.nix: every module, role and piece of hardware it offers,
# by module system, the packages it builds, and what each one is given.
# Modules live in modules/<tool>/, roles in roles/<role>/ and hardware in
# hardware/<model>/, one file per module system: home.nix for home-manager,
# darwin.nix, nixos.nix, or system.nix for one file that is both of the last
# two. A module's other files sit beside it. To see how any file fits, find
# its path here.
{
  description = "Joel's machines as a library: the modules and roles every machine builds from";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-darwin = {
      url = "github:nix-darwin/nix-darwin";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    wallpapers.url = "github:Jitsusama/wallpapers.nix";
    neovim-pi = {
      url = "github:Jitsusama/neovim.pi";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      home-manager,
      nix-darwin,
      treefmt-nix,
      wallpapers,
      neovim-pi,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
      ];
      forEachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      treefmt = pkgs: treefmt-nix.lib.evalModule pkgs ./treefmt.nix;

      # The module a file makes once it's given what it takes from here. The
      # key gives it the identity a path import has, so importing it twice
      # counts once and disabledModules can name it.
      moduleFrom = file: given: {
        key = toString file;
        imports = [ (nixpkgs.lib.modules.importApply file given) ];
      };
    in
    {
      homeModules = {
        bat = ./modules/bat/home.nix;
        broot = ./modules/broot/home.nix;
        btop = ./modules/btop/home.nix;
        claude = ./modules/claude/home.nix;
        curl = ./modules/curl/home.nix;
        dig = ./modules/dig/home.nix;
        direnv = ./modules/direnv/home.nix;
        fd = ./modules/fd/home.nix;
        fzf = ./modules/fzf/home.nix;
        gh = ./modules/gh/home.nix;
        ghostty = moduleFrom ./modules/ghostty/home.nix { inherit wallpapers; };
        git = ./modules/git/home.nix;
        glow = ./modules/glow/home.nix;
        gnupg = ./modules/gnupg/home.nix;
        home-manager = ./modules/home-manager/home.nix;
        homebrew = ./modules/homebrew/home.nix;
        identity = ./modules/identity/home.nix;
        jq = ./modules/jq/home.nix;
        lsd = ./modules/lsd/home.nix;
        neovim = moduleFrom ./modules/neovim/home.nix { inherit neovim-pi; };
        nixfmt = ./modules/nixfmt/home.nix;
        opencode = ./modules/opencode/home.nix;
        openssl = ./modules/openssl/home.nix;
        ripgrep = ./modules/ripgrep/home.nix;
        telnet = ./modules/telnet/home.nix;
        tree = ./modules/tree/home.nix;
        tree-sitter = ./modules/tree-sitter/home.nix;
        wezterm = moduleFrom ./modules/wezterm/home.nix { inherit wallpapers; };
        yq = ./modules/yq/home.nix;
        zellij = ./modules/zellij/home.nix;
        zoxide = ./modules/zoxide/home.nix;
        zsh = ./modules/zsh/home.nix;

        base = moduleFrom ./roles/base/home.nix { inherit (self) homeModules; };
        workstation = moduleFrom ./roles/workstation/home.nix { inherit (self) homeModules; };
      };

      darwinModules = {
        fonts = ./modules/fonts/system.nix;
        home-manager = moduleFrom ./modules/home-manager/darwin.nix { inherit home-manager; };
        homebrew = ./modules/homebrew/darwin.nix;
        macos-defaults = ./modules/macos-defaults/darwin.nix;
        nixpkgs = ./modules/nixpkgs/system.nix;
        zsh = ./modules/zsh/darwin.nix;

        base = moduleFrom ./roles/base/darwin.nix { inherit (self) darwinModules homeModules; };
        workstation = moduleFrom ./roles/workstation/darwin.nix {
          inherit (self) darwinModules homeModules;
        };
        graphical = moduleFrom ./roles/graphical/darwin.nix {
          inherit (self) darwinModules homeModules;
        };
      };

      nixosModules = {
        fonts = ./modules/fonts/system.nix;
        home-manager = moduleFrom ./modules/home-manager/nixos.nix { inherit home-manager; };
        kernel = ./modules/kernel/nixos.nix;
        keyboard = ./modules/keyboard/nixos.nix;
        keyd = ./modules/keyd/nixos.nix;
        memory = ./modules/memory/nixos.nix;
        nixpkgs = ./modules/nixpkgs/system.nix;
        perf = ./modules/perf/nixos.nix;
        power-profiles-daemon = ./modules/power-profiles-daemon/nixos.nix;

        base = moduleFrom ./roles/base/nixos.nix { inherit (self) nixosModules homeModules; };
        workstation = moduleFrom ./roles/workstation/nixos.nix {
          inherit (self) nixosModules homeModules;
        };
        graphical = moduleFrom ./roles/graphical/nixos.nix { inherit (self) nixosModules; };
        laptop = moduleFrom ./roles/laptop/nixos.nix { inherit (self) nixosModules; };

        dell-xps-14-da14260 = ./hardware/dell-xps-14-da14260/nixos.nix;
      };

      # What core.nix builds that a cache can hold apart from any machine: the
      # kernel for any x86-64 machine. A machine builds its own from the same
      # file, for its CPU.
      packages.x86_64-linux = {
        kernel = import ./modules/kernel/package.nix nixpkgs.legacyPackages.x86_64-linux {
          cpu = null;
          profile = null;
        };
      };

      formatter = forEachSystem (pkgs: (treefmt pkgs).config.build.wrapper);

      checks = forEachSystem (
        pkgs:
        import ./checks.nix {
          inherit
            self
            pkgs
            nixpkgs
            nix-darwin
            ;
          formatting = (treefmt pkgs).config.build.check self;
        }
      );

      # What dotfiles and the work repository import today: every module of
      # each class, in the order they were imported before the modules above
      # existed, so their machines build exactly as they did. The three that
      # take an input are applied directly rather than through moduleFrom,
      # whose extra level of imports would reorder lists such as Neovim's
      # plugins. These go once both import the modules and roles instead.
      home-manager.imports = [
        self.homeModules.bat
        self.homeModules.btop
        self.homeModules.broot
        self.homeModules.claude
        self.homeModules.curl
        self.homeModules.dig
        self.homeModules.direnv
        self.homeModules.fd
        self.homeModules.fzf
        self.homeModules.gh
        (import ./modules/ghostty/home.nix { inherit wallpapers; })
        self.homeModules.git
        self.homeModules.glow
        self.homeModules.gnupg
        self.homeModules.homebrew
        self.homeModules.jq
        self.homeModules.lsd
        (import ./modules/neovim/home.nix { inherit neovim-pi; })
        self.homeModules.nixfmt
        self.homeModules.opencode
        self.homeModules.openssl
        self.homeModules.ripgrep
        self.homeModules.telnet
        self.homeModules.tree
        self.homeModules.tree-sitter
        (import ./modules/wezterm/home.nix { inherit wallpapers; })
        self.homeModules.yq
        self.homeModules.zellij
        self.homeModules.zoxide
        self.homeModules.zsh
        self.homeModules.home-manager
      ];
      nix-darwin.imports = [
        self.darwinModules.fonts
        self.darwinModules.homebrew
        self.darwinModules.macos-defaults
        self.darwinModules.zsh
      ];
      inherit wallpapers neovim-pi;
    };
}
