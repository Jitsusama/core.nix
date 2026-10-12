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
    # Releases, since both decide whether a machine boots at all.
    lanzaboote = {
      # Past 1.2.0 for protectedSystem, which keeps the booted generation
      # installed so the disk's policy never loses PCR 4, and for loader
      # measurements that survive a boot loader update. Back to a tag once a
      # release carries both.
      url = "github:nix-community/lanzaboote/5bc59941ad05cfa434501ab20770bc6c6e824d4a";
      inputs.nixpkgs.follows = "nixpkgs";
      # Only lanzaboote's own CI uses it.
      inputs.pre-commit.follows = "";
    };
    disko = {
      url = "github:nix-community/disko/v1.13.0";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    wallpapers.url = "github:Jitsusama/wallpapers.nix";
    # Builds Spotify's client with the theme's colours in it.
    spicetify-nix = {
      url = "github:Gerg-L/spicetify-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    neovim-pi = {
      url = "github:Jitsusama/neovim.pi";
      flake = false;
    };
    # The ErgoDox's firmware: ZSA's QMK, the firmware24 branch its layout was
    # made for, and the two of its submodules a board with an AVR chip builds
    # with, at the commits that branch records for them. The three move
    # together, so they're pinned by commit rather than by branch.
    qmk = {
      url = "github:zsa/qmk_firmware/5456d00fd858d746df47b1f30e456075c6a4ba24";
      flake = false;
    };
    lufa = {
      url = "github:qmk/lufa/549b97320d515bfca2f95c145a67bd13be968faa";
      flake = false;
    };
    printf = {
      url = "github:qmk/printf/c2e3b4e10d281e7f0f694d3ecbd9f320977288cc";
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
      lanzaboote,
      disko,
      wallpapers,
      neovim-pi,
      spicetify-nix,
      qmk,
      lufa,
      printf,
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
        camera = ./modules/camera/home.nix;
        cargo = ./modules/cargo/home.nix;
        claude = ./modules/claude/home.nix;
        curl = ./modules/curl/home.nix;
        dig = ./modules/dig/home.nix;
        direnv = ./modules/direnv/home.nix;
        ergodox = moduleFrom ./modules/ergodox/home.nix { inherit qmk lufa printf; };
        fd = ./modules/fd/home.nix;
        fontconfig = ./modules/fontconfig/home.nix;
        fzf = ./modules/fzf/home.nix;
        gh = ./modules/gh/home.nix;
        ghostty = moduleFrom ./modules/ghostty/home.nix { inherit wallpapers; };
        gimp = moduleFrom ./modules/gimp/home.nix { inherit (self.homeModules) niri quickshell; };
        git = ./modules/git/home.nix;
        glow = ./modules/glow/home.nix;
        gnupg = ./modules/gnupg/home.nix;
        gtk = ./modules/gtk/home.nix;
        home-manager = ./modules/home-manager/home.nix;
        homebrew = ./modules/homebrew/home.nix;
        identity = ./modules/identity/home.nix;
        inkscape = moduleFrom ./modules/inkscape/home.nix { inherit (self.homeModules) niri; };
        jq = ./modules/jq/home.nix;
        keys = ./modules/keys/home.nix;
        kitty = ./modules/kitty/home.nix;
        lsd = ./modules/lsd/home.nix;
        mpv = ./modules/mpv/home.nix;
        neovim = moduleFrom ./modules/neovim/home.nix { inherit neovim-pi; };
        niri = ./modules/niri/home.nix;
        nixfmt = ./modules/nixfmt/home.nix;
        opencode = ./modules/opencode/home.nix;
        openssl = ./modules/openssl/home.nix;
        podman = ./modules/podman/home.nix;
        quickshell = moduleFrom ./modules/quickshell/home.nix { inherit wallpapers; };
        ripgrep = ./modules/ripgrep/home.nix;
        signing = ./modules/signing/home.nix;
        spotify = moduleFrom ./modules/spotify/home.nix { inherit spicetify-nix; };
        ssh-tpm-agent = ./modules/ssh-tpm-agent/home.nix;
        swayidle = ./modules/swayidle/home.nix;
        swayimg = moduleFrom ./modules/swayimg/home.nix { inherit (self.homeModules) niri quickshell; };
        telnet = ./modules/telnet/home.nix;
        theme = ./modules/theme/home.nix;
        tree = ./modules/tree/home.nix;
        tree-sitter = ./modules/tree-sitter/home.nix;
        tuple = moduleFrom ./modules/tuple/home.nix { inherit (self.homeModules) niri quickshell; };
        wezterm = moduleFrom ./modules/wezterm/home.nix { inherit wallpapers; };
        yq = ./modules/yq/home.nix;
        zathura = ./modules/zathura/home.nix;
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
        yubikey = ./modules/yubikey/system.nix;
        zsh = ./modules/zsh/system.nix;

        base = moduleFrom ./roles/base/darwin.nix { inherit (self) darwinModules homeModules; };
        workstation = moduleFrom ./roles/workstation/darwin.nix {
          inherit (self) darwinModules homeModules;
        };
        graphical = moduleFrom ./roles/graphical/darwin.nix {
          inherit (self) darwinModules homeModules;
        };
      };

      nixosModules = {
        bluetooth = ./modules/bluetooth/nixos.nix;
        btrfs = ./modules/btrfs/nixos.nix;
        camera = ./modules/camera/nixos.nix;
        coredump = ./modules/coredump/nixos.nix;
        disko = moduleFrom ./modules/disko/nixos.nix { inherit disko; };
        ergodox = ./modules/ergodox/nixos.nix;
        firmware-settings = ./modules/firmware-settings/nixos.nix;
        fonts = ./modules/fonts/system.nix;
        gnome-keyring = ./modules/gnome-keyring/nixos.nix;
        gpu = ./modules/gpu/nixos.nix;
        greetd = ./modules/greetd/nixos.nix;
        home-manager = moduleFrom ./modules/home-manager/nixos.nix { inherit home-manager; };
        kernel = ./modules/kernel/nixos.nix;
        keyboard = ./modules/keyboard/nixos.nix;
        keyd = ./modules/keyd/nixos.nix;
        lanzaboote = moduleFrom ./modules/lanzaboote/nixos.nix { inherit lanzaboote; };
        libvirt = ./modules/libvirt/nixos.nix;
        memory = ./modules/memory/nixos.nix;
        mpv = ./modules/mpv/nixos.nix;
        networkmanager = ./modules/networkmanager/nixos.nix;
        nftables = ./modules/nftables/nixos.nix;
        niri = ./modules/niri/nixos.nix;
        nix-daemon = ./modules/nix-daemon/nixos.nix;
        nixpkgs = ./modules/nixpkgs/system.nix;
        onepassword = ./modules/onepassword/nixos.nix;
        perf = ./modules/perf/nixos.nix;
        pipewire = ./modules/pipewire/nixos.nix;
        podman = ./modules/podman/nixos.nix;
        power-profiles-daemon = ./modules/power-profiles-daemon/nixos.nix;
        quickshell = ./modules/quickshell/nixos.nix;
        slices = ./modules/slices/nixos.nix;
        swayimg = ./modules/swayimg/nixos.nix;
        systemd-networkd = ./modules/systemd-networkd/nixos.nix;
        tpm = ./modules/tpm/nixos.nix;
        wifid = ./modules/wifid/nixos.nix;
        wpa_supplicant = ./modules/wpa_supplicant/nixos.nix;
        yubikey = ./modules/yubikey/system.nix;
        zathura = ./modules/zathura/nixos.nix;
        zsh = ./modules/zsh/nixos.nix;

        base = moduleFrom ./roles/base/nixos.nix { inherit (self) nixosModules homeModules; };
        workstation = moduleFrom ./roles/workstation/nixos.nix {
          inherit (self) nixosModules homeModules;
        };
        graphical = moduleFrom ./roles/graphical/nixos.nix {
          inherit (self) nixosModules homeModules;
        };
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
      # The tools for working on core's own programs, so nothing but Nix has
      # to be installed first. direnv enters it in a program's directory.
      devShells = forEachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.cargo
            pkgs.clippy
            pkgs.rust-analyzer
            pkgs.rustc
            pkgs.rustfmt
          ];
        };
      });

      # Where core.nix's own programs are worked on, with the toolchain
      # their packages build with: `nix develop`, then cargo in the
      # program's directory.
      devShells.x86_64-linux.default =
        let
          pkgs = nixpkgs.legacyPackages.x86_64-linux;
        in
        pkgs.mkShell {
          inputsFrom = [ (pkgs.callPackage ./modules/wifid/package.nix { }) ];
          packages = [
            pkgs.clippy
            pkgs.rustfmt
          ];
        };

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

      # What machine repositories imported before the modules above existed:
      # every module of each class, in the order they were imported then, so
      # their machines build exactly as they did. The three that
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
