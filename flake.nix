# The index of core.nix: every module, role and piece of hardware it offers,
# by module system, the packages it builds, and what each one is given.
# Modules live in modules/<tool>/, roles in roles/<role>/ and hardware in
# hardware/<model>/, one file per module system: home.nix for home-manager,
# darwin.nix, nixos.nix, or system.nix for one file that is both of the last
# two, and treefmt.nix for how a repository is formatted and linted. A
# module's other files sit beside it. To see how any file fits, find
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
      inputs.rust-overlay.follows = "rust-overlay";
      # Only lanzaboote's own CI uses it.
      inputs.pre-commit.follows = "";
    };
    # Rust's standard library for other platforms, so a check can compile
    # the disk guard for macOS where no Mac runs.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
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
    # The node that carries the harness mesh between a realm's machines.
    mycelium = {
      url = "github:Jitsusama/mycelium";
      inputs.nixpkgs.follows = "nixpkgs";
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
      rust-overlay,
      disko,
      wallpapers,
      neovim-pi,
      mycelium,
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
      # Core holds itself to the linters it offers, and to what only it has.
      treefmt =
        pkgs:
        treefmt-nix.lib.evalModule pkgs {
          imports = [
            self.treefmtModules.treefmt
            ./treefmt.nix
          ];
        };

      # The module a file makes once it's given what it takes from here. The
      # key gives it the identity a path import has, so importing it twice
      # counts once and disabledModules can name it.
      moduleFrom = file: given: {
        key = toString file;
        imports = [ (nixpkgs.lib.modules.importApply file given) ];
      };

      # The stick's own system, which installs and repairs a machine.
      stick = nixpkgs.lib.nixosSystem {
        modules = [
          self.nixosModules.guide-stick
          { nixpkgs.hostPlatform = "x86_64-linux"; }
        ];
      };
    in
    {
      homeModules = {
        backlight = ./modules/backlight/home.nix;
        bat = ./modules/bat/home.nix;
        broot = ./modules/broot/home.nix;
        btop = ./modules/btop/home.nix;
        camera = ./modules/camera/home.nix;
        cargo = ./modules/cargo/home.nix;
        claude = ./modules/claude/home.nix;
        curl = ./modules/curl/home.nix;
        delta = ./modules/delta/home.nix;
        dictation = moduleFrom ./modules/dictation/home.nix { inherit (self.homeModules) quickshell; };
        dig = ./modules/dig/home.nix;
        direnv = ./modules/direnv/home.nix;
        disk-guard = ./modules/disk-guard/home.nix;
        eero = ./modules/eero/home.nix;
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
        google = moduleFrom ./modules/google/home.nix { inherit (self.homeModules) quickshell; };
        gtk = ./modules/gtk/home.nix;
        home-manager = ./modules/home-manager/home.nix;
        homebrew = ./modules/homebrew/home.nix;
        identity = ./modules/identity/home.nix;
        inkscape = moduleFrom ./modules/inkscape/home.nix { inherit (self.homeModules) niri; };
        inlyne = ./modules/inlyne/home.nix;
        jq = ./modules/jq/home.nix;
        keys = ./modules/keys/home.nix;
        kitty = ./modules/kitty/home.nix;
        lag-watch = ./modules/lag-watch/home.nix;
        load-proof = ./modules/load-proof/home.nix;
        lsd = ./modules/lsd/home.nix;
        mpv = ./modules/mpv/home.nix;
        mycelium = moduleFrom ./modules/mycelium/home.nix { inherit mycelium; };
        neovim = moduleFrom ./modules/neovim/home.nix { inherit neovim-pi; };
        network-revive = ./modules/network-revive/home.nix;
        niri = ./modules/niri/home.nix;
        nixfmt = ./modules/nixfmt/home.nix;
        open-with = ./modules/open-with/home.nix;
        opencode = ./modules/opencode/home.nix;
        openssl = ./modules/openssl/home.nix;
        pi = ./modules/pi/home.nix;
        pi-packages = ./modules/pi-packages/home.nix;
        pnpm = ./modules/pnpm/home.nix;
        podman = ./modules/podman/home.nix;
        prompt = ./modules/prompt/home.nix;
        qt = ./modules/qt/home.nix;
        quickshell = moduleFrom ./modules/quickshell/home.nix { inherit wallpapers; };
        ripgrep = ./modules/ripgrep/home.nix;
        secure-enclave-signing = ./modules/secure-enclave-signing/home.nix;
        session-backup = ./modules/session-backup/home.nix;
        signing = ./modules/signing/home.nix;
        spend-watch = ./modules/spend-watch/home.nix;
        spotify = moduleFrom ./modules/spotify/home.nix { inherit spicetify-nix; };
        ssh-tpm-agent = ./modules/ssh-tpm-agent/home.nix;
        swayidle = ./modules/swayidle/home.nix;
        swayimg = moduleFrom ./modules/swayimg/home.nix { inherit (self.homeModules) niri quickshell; };
        telnet = ./modules/telnet/home.nix;
        theme = ./modules/theme/home.nix;
        tpm-signing = ./modules/tpm-signing/home.nix;
        tree = ./modules/tree/home.nix;
        tree-sitter = ./modules/tree-sitter/home.nix;
        tuple = moduleFrom ./modules/tuple/home.nix { inherit (self.homeModules) niri quickshell; };
        vivid = ./modules/vivid/home.nix;
        wezterm = moduleFrom ./modules/wezterm/home.nix { inherit wallpapers; };
        wifid = ./modules/wifid/home.nix;
        yq = ./modules/yq/home.nix;
        zathura = ./modules/zathura/home.nix;
        zoxide = ./modules/zoxide/home.nix;
        zsh = ./modules/zsh/home.nix;

        base = moduleFrom ./roles/base/home.nix { inherit (self) homeModules; };
        workstation = moduleFrom ./roles/workstation/home.nix { inherit (self) homeModules; };
      };

      darwinModules = {
        disk-guard = ./modules/disk-guard/darwin.nix;
        fonts = ./modules/fonts/system.nix;
        home-manager = moduleFrom ./modules/home-manager/darwin.nix { inherit home-manager; };
        homebrew = ./modules/homebrew/darwin.nix;
        macos-defaults = ./modules/macos-defaults/darwin.nix;
        nixpkgs = ./modules/nixpkgs/system.nix;
        sudo = ./modules/sudo/darwin.nix;
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
        backlight = ./modules/backlight/nixos.nix;
        bluetooth = ./modules/bluetooth/nixos.nix;
        bluetooth-links = ./modules/bluetooth-links/nixos.nix;
        btrfs = ./modules/btrfs/nixos.nix;
        camera = ./modules/camera/nixos.nix;
        console = ./modules/console/nixos.nix;
        coredump = ./modules/coredump/nixos.nix;
        creative-pebble-x-plus = ./modules/creative-pebble-x-plus/nixos.nix;
        ddcutil = ./modules/ddcutil/nixos.nix;
        desk-facts = ./modules/desk-facts/nixos.nix;
        device-check = ./modules/device-check/nixos.nix;
        dictation = ./modules/dictation/nixos.nix;
        disk-guard = ./modules/disk-guard/nixos.nix;
        disk-loss-panic = ./modules/disk-loss-panic/nixos.nix;
        disko = moduleFrom ./modules/disko/nixos.nix { inherit disko; };
        ergodox = ./modules/ergodox/nixos.nix;
        firmware-settings = ./modules/firmware-settings/nixos.nix;
        fonts = ./modules/fonts/nixos.nix;
        gnome-keyring = ./modules/gnome-keyring/nixos.nix;
        gpu = ./modules/gpu/nixos.nix;
        greetd = ./modules/greetd/nixos.nix;
        guide = ./modules/guide/nixos.nix;
        home-manager = moduleFrom ./modules/home-manager/nixos.nix { inherit home-manager; };
        inlyne = ./modules/inlyne/nixos.nix;
        ir-camera = ./modules/ir-camera/nixos.nix;
        kernel = ./modules/kernel/nixos.nix;
        keyboard = ./modules/keyboard/nixos.nix;
        keyd = ./modules/keyd/nixos.nix;
        kmscon = ./modules/kmscon/nixos.nix;
        lanzaboote = moduleFrom ./modules/lanzaboote/nixos.nix { inherit lanzaboote; };
        libvirt = ./modules/libvirt/nixos.nix;
        lid = ./modules/lid/nixos.nix;
        linux-id = ./modules/linux-id/nixos.nix;
        litra = ./modules/litra/nixos.nix;
        memory = ./modules/memory/nixos.nix;
        memory-modules = ./modules/memory-modules/nixos.nix;
        mic-mute-light = ./modules/mic-mute-light/nixos.nix;
        mpv = ./modules/mpv/nixos.nix;
        network-revive = ./modules/network-revive/nixos.nix;
        nftables = ./modules/nftables/nixos.nix;
        niri = ./modules/niri/nixos.nix;
        niri-off-cpu = ./modules/niri-off-cpu/nixos.nix;
        nix-daemon = ./modules/nix-daemon/nixos.nix;
        nixpkgs = ./modules/nixpkgs/system.nix;
        onepassword = ./modules/onepassword/nixos.nix;
        open-with = ./modules/open-with/nixos.nix;
        panel = ./modules/panel/nixos.nix;
        perf = ./modules/perf/nixos.nix;
        pipewire = ./modules/pipewire/nixos.nix;
        plymouth = moduleFrom ./modules/plymouth/nixos.nix { inherit wallpapers; };
        podman = ./modules/podman/nixos.nix;
        power-profiles-daemon = ./modules/power-profiles-daemon/nixos.nix;
        quickshell = ./modules/quickshell/nixos.nix;
        shaper = ./modules/shaper/nixos.nix;
        shure-mv7 = ./modules/shure-mv7/nixos.nix;
        slices = ./modules/slices/nixos.nix;
        swayimg = ./modules/swayimg/nixos.nix;
        switch-system = ./modules/switch-system/nixos.nix;
        systemd = ./modules/systemd/nixos.nix;
        systemd-networkd = ./modules/systemd-networkd/nixos.nix;
        tcp = ./modules/tcp/nixos.nix;
        timesyncd = ./modules/timesyncd/nixos.nix;
        touchpad = ./modules/touchpad/nixos.nix;
        tpm = ./modules/tpm/nixos.nix;
        usb-camera-holders = ./modules/usb-camera-holders/nixos.nix;
        usb-wake = ./modules/usb-wake/nixos.nix;
        v4l2loopback = ./modules/v4l2loopback/nixos.nix;
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
        laptop = moduleFrom ./roles/laptop/nixos.nix { inherit (self) nixosModules homeModules; };

        # The stick's own system, which installs and repairs a machine.
        guide-stick = moduleFrom ./modules/guide/stick.nix {
          inherit (self) nixosModules;
          inherit nixpkgs;
        };

        dell-xps-14-da14260 = ./hardware/dell-xps-14-da14260/nixos.nix;
        microsoft-surface-laptop-4-1958 = ./hardware/microsoft-surface-laptop-4-1958/nixos.nix;
      };

      # What core.nix builds that a cache can hold apart from any machine: the
      # kernel for any x86-64 machine, the guide that installs and repairs
      # one, and the image of the stick it runs on. A machine builds its own
      # kernel from the same file, for its CPU.
      packages.x86_64-linux = {
        guide = nixpkgs.legacyPackages.x86_64-linux.callPackage ./modules/guide/package.nix { };
        stick = stick.config.system.build.image;
        kernel = import ./modules/kernel/package.nix nixpkgs.legacyPackages.x86_64-linux {
          cpu = null;
          profile = null;
        };
      };

      # A machine's guide, for its own repository to offer as `nix run
      # .#guide`: the stick's image and the machine's payload built in, so
      # writing its stick on a host needs nothing more than plugging one in.
      # The host is an x86-64 Linux machine; a Mac's guide is to come.
      lib.guide =
        {
          # The machine's configuration, nixosSystem's config.
          config,
          # Its YubiKeys by the names the person knows them by.
          yubikeys,
          # The account the person signs in with, when it has no greeter
          # to name it.
          account ? null,
        }:
        let
          pkgs = nixpkgs.legacyPackages.x86_64-linux;
          guide = pkgs.callPackage ./modules/guide/package.nix { };
          payload = pkgs.callPackage ./modules/guide/payload.nix { } { inherit config yubikeys account; };
          image = "${stick.config.system.build.image}/${stick.config.image.filePath}";
        in
        pkgs.runCommand "guide-${config.networking.hostName}"
          {
            nativeBuildInputs = [ pkgs.makeBinaryWrapper ];
            meta.mainProgram = "guide";
          }
          ''
            makeBinaryWrapper ${pkgs.lib.getExe guide} $out/bin/guide --add-flags "host ${image} --install ${payload}"
          '';

      # How a repository built on core holds its Rust to clippy as core does:
      # `(core.lib.clippy pkgs).passes { name = ...; package = ...; }` is a
      # check that the package passes, with its lints and core's settings.
      lib.clippy = pkgs: pkgs.callPackage ./modules/clippy/clippy.nix { };

      # How a repository built on core holds its docs to naming only what
      # exists, as core does: `core.lib.docsNameWhatExists pkgs { src = ./.;
      # exempt = [ ... ]; }` is a check that fails on a link or a backticked
      # path from one of its top folders that names nothing.
      lib.docsNameWhatExists = pkgs: pkgs.callPackage ./tests/docs-name-what-exists.nix { };

      # How a repository built on core holds every folder to saying what it
      # holds, as core does: `core.lib.everyFolderSaysWhatItHolds pkgs { src =
      # ./.; waiting = [ ... ]; }` is a check that fails on a folder with no
      # note, and on a waiting one that has its note or is gone.
      lib.everyFolderSaysWhatItHolds =
        pkgs: pkgs.callPackage ./tests/every-folder-says-what-it-holds.nix { };

      # How a repository built on core holds its names to core's glossary:
      # `core.lib.namesSayTheGlossary pkgs { names = ...; otherSenses = ...;
      # waiting = ...; }` is a check that no name says a name the glossary
      # replaces, but those it keeps, each with why.
      lib.namesSayTheGlossary = pkgs: pkgs.callPackage ./tests/names.nix { };

      # What VM tests' scripts share, for a repository built on core to give
      # its own: `extraPythonPackages = core.lib.vmHelpers pkgs;` lets a
      # script import vm_helpers, and core's linters read its types.
      lib.vmHelpers = pkgs: python: [ (pkgs.callPackage ./tests/lib/vm-helpers.nix { } python) ];

      # How a repository built on core formats and lints itself as core does,
      # imported into the module it gives treefmt-nix.
      treefmtModules = {
        treefmt = ./modules/treefmt/treefmt.nix;
      };

      formatter = forEachSystem (pkgs: (treefmt pkgs).config.build.wrapper);
      # The tools for working on core's own programs, the disk guard and
      # wifid among them, with the toolchain their packages build with, so
      # nothing but Nix has to be installed first. direnv enters it anywhere in
      # the repository (.envrc), or `nix develop` then cargo in a program's
      # directory. It also holds shots-changed, which compares two runs of the
      # desktop test's pictures, changed-checks, which names and builds the
      # checks a change touches, and on Linux run-subtests, which runs some
      # of a VM test's subtests in seconds, and shell-preview, which shows the
      # shell from the working copy in a niri window (docs/testing.md).
      devShells = forEachSystem (pkgs: {
        default = pkgs.mkShell {
          # wifid's sign-in window builds against GTK and WebKit, which
          # only Linux has.
          inputsFrom = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            (pkgs.callPackage ./modules/wifid/sign-in/package.nix { })
          ];
          packages = [
            pkgs.cargo
            pkgs.clippy
            pkgs.rust-analyzer
            pkgs.rustc
            pkgs.rustfmt
            (pkgs.callPackage ./tools/changed-checks/package.nix { })
            (pkgs.callPackage ./tools/shots-changed/shots-changed.nix { })
          ]
          # The guide's tests check the sticks it writes with sfdisk,
          # fsck.fat and mtools, and its signatures against sbsign's.
          # VM tests boot under QEMU, which they need Linux for.
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            (pkgs.callPackage ./tools/run-subtests/run-subtests.nix { })
            (pkgs.callPackage ./tools/shell-preview/package.nix { })
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            pkgs.dosfstools
            pkgs.mtools
            pkgs.openssl
            pkgs.sbsigntool
            pkgs.util-linux
          ];
          # A guide built here finds its tools as the package's does.
          env = pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux (
            {
              GUIDE_TEST_EFI = "${pkgs.systemd}/lib/systemd/boot/efi";
            }
            // import ./modules/guide/tools.nix { inherit pkgs; }
          );
        };
      });

      checks = forEachSystem (
        pkgs:
        import ./checks.nix {
          inherit
            self
            pkgs
            nixpkgs
            nix-darwin
            rust-overlay
            ;
          formatting = (treefmt pkgs).config.build.check self;
        }
      );
    };
}
