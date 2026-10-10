# Everything `nix flake check` runs, from the bottom of the pyramid up: the
# promises modules and roles make, read from evaluated machines; the example
# machines, which use core only through its outputs and so evaluate every
# module there is; the real programs reading the files an account gets; and a
# few virtual machines, each showing what nothing cheaper can. Each check has
# to give a signal the ones below it don't, and docs/testing.md says which.
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
          # A module that lays out the disk replaces this.
          fileSystems."/" = lib.mkDefault {
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

  # The line a kernel setting leaves in the finished configuration.
  kernelConfigLine =
    option: setting:
    if setting ? freeform then
      "CONFIG_${option}=${setting.freeform}"
    else if setting.tristate == "n" then
      "# CONFIG_${option} is not set"
    else
      "CONFIG_${option}=${setting.tristate}";

  # The kernel's patches apply, its configuration resolves, and every setting
  # in settings.nix is in the result: Kconfig quietly drops a setting whose
  # dependencies aren't met. An optional setting also holds when the kernel
  # doesn't offer its option at all. Only the configuration is built, in
  # minutes; the kernel itself takes far longer than a check should.
  kernelSettingsHold =
    name: kernel:
    let
      settings = import ./modules/kernel/settings.nix { inherit lib; };
      expect =
        option: setting:
        lib.escapeShellArgs [
          "expect"
          (kernelConfigLine option setting)
          option
          (if setting.optional or false then "optional" else "required")
        ];
    in
    pkgs.runCommandLocal name { } ''
      config=${kernel.configfile}
      failed=
      expect() {
        grep -qxF "$1" "$config" && return
        [ "$3" = optional ] && ! grep -qE "^(# )?CONFIG_$2[= ]" "$config" && return
        echo "Kernel setting didn't hold: $1"
        failed=1
      }
      ${lib.concatLines (lib.mapAttrsToList expect settings)}
      [ -z "$failed" ] && touch $out
    '';

  # The machines most checks read, each evaluated once and shared, since an
  # evaluation is most of what a check costs.
  workstation = bareNixos [ self.nixosModules.workstation ];
  graphical = bareNixos [ self.nixosModules.graphical ];
  macWorkstation = bareDarwin [ self.darwinModules.workstation ];

  # The virtual machine tests, which CI runs one to a runner beside the rest.
  # Each has to show something nothing cheaper can.
  vmTests = {
    # The disk layout and Secure Boot decide whether a machine starts at all.
    vm-secure-boot-installs = import ./tests/secure-boot.nix { inherit self pkgs; };
    vm-commits-are-signed = import ./tests/signing.nix { inherit self pkgs; };
    vm-keyring-opens-without-asking = import ./tests/gnome-keyring.nix { inherit self pkgs; };
    vm-speakers-are-tuned = import ./tests/speakers.nix { inherit pkgs; };
    vm-desktop-works = import ./tests/desktop.nix { inherit self pkgs; };
  };
in
{
  inherit formatting;

  # Roles import the roles beneath them, so a machine importing two roles
  # imports the shared ones twice. That has to count once.
  nixos-role-twice-is-once =
    same "nixos-role-twice-is-once" "Importing a NixOS role twice changed the machine." workstation
      (bareNixos [
        self.nixosModules.workstation
        self.nixosModules.workstation
      ]);
  darwin-role-twice-is-once =
    same "darwin-role-twice-is-once" "Importing a nix-darwin role twice changed the machine."
      macWorkstation
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

  # A workstation with a screen has one SSH agent, the TPM's, though niri
  # brings gnome-keyring and its agent.
  one-ssh-agent =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
      ];
    in
    holds "one-ssh-agent" "gcr's SSH agent runs beside ssh-tpm-agent." (
      !machine.config.services.gnome.gcr-ssh-agent.enable
      && (account machine).services.ssh-tpm-agent.enable
    );

  # 1Password's Chrome extension talks to the desktop app through a helper in
  # the onepassword group, and every account may unlock the app with its own
  # password.
  onepassword-works-with-chrome =
    holds "onepassword-works-with-chrome" "1Password lacks its browser helper or its polkit owners."
      (
        graphical.config.security.wrappers."1Password-BrowserSupport".group or null == "onepassword"
        && graphical.config.programs._1password-gui.polkitPolicyOwners == [ "someone" ]
      );

  # home-manager configures zsh in every account but leaves the login shell to
  # the system, so an account on NixOS has to be given zsh to read any of it.
  # A Mac's accounts log in to zsh already.
  zsh-is-the-login-shell =
    holds "zsh-is-the-login-shell" "An account on NixOS doesn't log in to zsh."
      (lib.getName (bareNixos [ self.nixosModules.base ]).config.users.users.someone.shell == "zsh");

  # The YubiKey gets Joel's keys back on any machine he writes code on, so both
  # workstation roles bring its tools, a Mac as much as Linux.
  yubikey-on-every-workstation =
    let
      hasYkman =
        machine:
        lib.any (
          package: lib.getName package == "yubikey-manager"
        ) machine.config.environment.systemPackages;
    in
    holds "yubikey-on-every-workstation" "A workstation lacks the YubiKey's tools." (
      hasYkman workstation && hasYkman macWorkstation
    );

  # Every switch keeps the generation the machine booted on the ESP, so
  # systemd-pcrlock can always account for the running boot and the disk's
  # policy keeps PCR 4.
  booted-generation-stays-installed =
    let
      machine = bareNixos [ self.nixosModules.lanzaboote ];
    in
    holds "booted-generation-stays-installed"
      "lanzaboote can remove the booted generation, which drops PCR 4 from the disk's policy."
      (machine.config.boot.lanzaboote.protectedSystem == "/run/booted-system");

  # A theme that draws text too faint to read doesn't build. The ratio is
  # WCAG's: black on white is 21:1, and #777777 on white just misses 4.5:1.
  # Osaka Jade's own muted, at 2.90:1, is the faint text it was made to catch.
  a-faint-theme-is-refused =
    let
      contrast = import ./modules/theme/contrast.nix { inherit lib; };
      machine = bareNixos [
        {
          home-manager.users.someone = {
            imports = [ ./modules/theme/home.nix ];
            jitsusama.theme.roles.muted = "#53685B";
          };
        }
      ];
      refusals = map (check: check.message) (
        lib.filter (check: !check.assertion) (account machine).assertions
      );
    in
    holds "a-faint-theme-is-refused" "A theme with text too faint to read wasn't refused." (
      contrast.show (contrast.ratio "#000000" "#ffffff") == "21.00"
      && contrast.show (contrast.ratio "#777777" "#ffffff") == "4.47"
      && lib.any (lib.hasInfix "muted on background: 2.90:1") refusals
    );

  # A switch restarts Quickshell when any file it reads changes, so the shell
  # on screen is never the one from before the switch.
  quickshell-restarts-with-its-files =
    let
      home = account graphical;
      files = lib.filterAttrs (name: _: lib.hasPrefix "quickshell/" name) home.xdg.configFile;
      triggers = home.systemd.user.services.quickshell.Unit.X-Restart-Triggers or [ ];
    in
    holds "quickshell-restarts-with-its-files"
      "A Quickshell file isn't among its unit's restart triggers, so a switch would leave the old one on screen."
      (files != { } && lib.all (file: lib.elem "${file.source}" triggers) (lib.attrValues files));

  # A chord two layers both claim never reaches the lower one, so a machine
  # with one doesn't build. The clashes here are spelt the way each program
  # spells its chords, so the map has to see through the spelling: kitty's
  # ctrl+alt+n is zellij's Ctrl Alt n, and its super+return is niri's
  # Super+Return. A layer claiming one chord twice is refused too.
  a-claimed-chord-is-refused =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        {
          home-manager.users.someone.jitsusama.kitty.keys = lib.mkAfter [
            {
              chord = "ctrl+alt+n";
              does = "Open a window";
              action = "new_os_window";
            }
            {
              chord = "super+return";
              does = "Open a tab";
              action = "new_tab";
            }
            {
              chord = "ctrl+shift+c";
              does = "Copy again";
              action = "copy_to_clipboard";
            }
          ];
        }
      ];
      refusals = lib.concatMapStrings (check: check.message) (
        lib.filter (check: !check.assertion) (account machine).assertions
      );
      fine =
        lib.all (check: check.assertion)
          (account (bareNixos [
            self.nixosModules.workstation
            self.nixosModules.graphical
          ])).assertions;
    in
    holds "a-claimed-chord-is-refused"
      "A chord two layers claim, or one layer claims twice, wasn't refused."
      (
        fine
        && lib.hasInfix "ctrl+alt+n: terminal (Open a window) takes it from multiplexer (Open a pane)" refusals
        && lib.hasInfix "super+enter: desktop (Open a terminal) takes it from terminal (Open a tab)" refusals
        && lib.hasInfix "ctrl+shift+c in terminal: Copy, Copy again" refusals
      );

  # Picking another theme changes everything the account draws: the desktop
  # rendered in Flexoki Light keeps none of Osaka Jade's colours anywhere in
  # its files, so no surface was left behind in the old one.
  a-theme-changes-everything =
    let
      osakaJade = (builtins.fromTOML (builtins.readFile ./modules/theme/osaka-jade.toml)).palette;
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { home-manager.users.someone.jitsusama.theme.name = "flexoki-light"; }
      ];
      home = account machine;
    in
    pkgs.runCommandLocal "a-theme-changes-everything" { nativeBuildInputs = [ pkgs.ripgrep ]; } ''
      test ${lib.escapeShellArg home.jitsusama.theme.roles.background} = '#FFFCF0'
      if rg --follow --ignore-case --fixed-strings --line-number --no-heading --hidden \
        ${
          lib.concatMapStringsSep " " (colour: "-e ${lib.escapeShellArg colour}") (lib.attrValues osakaJade)
        } ${home.home-files}/; then
        echo "The files above kept Osaka Jade's colours when the theme changed."
        exit 1
      fi
      touch $out
    '';

  # Every surface takes its look from the theme, so no module but the theme's
  # names a colour or a face of its own: one that did would stay put when the
  # theme changes. Colours are caught as hex or CSS functions, faces by the
  # families anything here installs or falls back to.
  nothing-names-its-own-look =
    let
      colours = [
        "#[0-9A-Fa-f]{6}([0-9A-Fa-f]{2})?\\b"
        "\\b(rgba?|hsla?)\\("
      ];
      faces = [
        "Monaspice"
        "Monaspace"
        "Mona Sans"
        "Hubot Sans"
        "Fira Code"
        "Fira Mono"
        "Victor Mono"
        "JetBrains Mono"
        "Nerd Font"
        "Adwaita Sans"
        "Adwaita Mono"
        "DejaVu"
        "Noto Sans"
        "Terminus"
      ];
      pattern = lib.concatStringsSep "|" (colours ++ [ "\\b(${lib.concatStringsSep "|" faces})\\b" ]);
    in
    pkgs.runCommandLocal "nothing-names-its-own-look" { nativeBuildInputs = [ pkgs.ripgrep ]; } ''
      cd ${./.}
      if rg --line-number --no-heading --hidden --glob '!modules/theme/**' \
        -e ${lib.escapeShellArg pattern} modules roles hardware tests examples flake.nix; then
        echo "The lines above name a colour or a face; name the theme's role or font instead."
        exit 1
      fi
      touch $out
    '';

  # A disk's partitions and its open volume go by its name, so a machine's
  # drive and a stick attached beside it are never taken for each other.
  disks-go-by-their-name =
    let
      machine = bareNixos [
        self.nixosModules.disko
        {
          jitsusama.disk = {
            name = "stick";
            device = "/dev/disk/by-id/usb-example";
            swapSize = "4G";
          };
        }
      ];
      inherit (machine.config.disko.devices.disk.main.content.partitions) boot system;
    in
    holds "disks-go-by-their-name" "A disk's partitions or volume don't carry its name." (
      boot.label == "stick-boot"
      && system.label == "stick"
      && machine.config.boot.initrd.luks.devices ? stick
    );

  # The kernel protects session.slice's memory only as far as every slice
  # above it is protected, so a floor one of them lacks is a floor nobody has,
  # and nothing says so.
  session-memory-is-protected =
    let
      inherit (graphical.config) systemd;
      floors = [
        systemd.user.slices.session.sliceConfig.MemoryLow or null
        systemd.slices.user.sliceConfig.MemoryLow or null
        systemd.slices."user-".sliceConfig.MemoryLow or null
        systemd.services."user@".serviceConfig.MemoryLow or null
      ];
    in
    holds "session-memory-is-protected"
      "A slice above session.slice lacks its memory floor, so session.slice has none."
      (lib.all (floor: floor != null && floor == lib.head floors) floors);

  example-nixos = evaluates "example-nixos" (
    import ./examples/nixos.nix {
      core = self;
      inherit nixpkgs;
    }
  );
  example-laptop = evaluates "example-laptop" (
    import ./examples/laptop.nix {
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

  # core.nix knows nothing about what builds on it: no file names a machine, a
  # machine repository or an employer, so the repositories that use core name
  # it and never the other way round. Every file is read but this one, which
  # has to name the words, and the lock file, which only names pins.
  stands-alone =
    let
      outside = [
        "shopify"
        "/opt/dev"
        "gitstream"
        "devx"
        "minerva"
        "cloudflare"
        "dotfiles"
        "joel-gerber"
        "joel.gerber"
        "grrbrr"
        "monorepo"
        "work repository"
        "work layer"
        "personal repository"
        "personal layer"
        "optimus"
        "penelope"
        "methuselah"
        "equilibrius"
      ];
    in
    pkgs.runCommandLocal "stands-alone" { nativeBuildInputs = [ pkgs.ripgrep ]; } ''
      cd ${./.}
      if rg --ignore-case --fixed-strings --line-number --no-heading --hidden \
        --glob '!checks.nix' --glob '!flake.lock' \
        ${lib.concatMapStringsSep " " (word: "-e ${lib.escapeShellArg word}") outside} .; then
        echo "The lines above name what builds on core.nix; they belong where it is built."
        exit 1
      fi
      touch $out
    '';
}
// lib.optionalAttrs (pkgs.stdenv.hostPlatform.system == "x86_64-linux") (
  lib.mapAttrs kernelSettingsHold {
    kernel-settings-hold = self.packages.x86_64-linux.kernel;

    # Each piece of hardware builds its own kernel, with its own patches.
    kernel-settings-hold-dell-xps-14-da14260 =
      (bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.boot.kernelPackages.kernel;
  }
  // {
    # niri reads back the files an account gets and rejects anything it
    # wouldn't load, from a misspelt action to a colour it can't parse.
    niri-accepts-its-configuration =
      let
        files = (account graphical).xdg.configFile;
      in
      pkgs.runCommandLocal "niri-accepts-its-configuration" { nativeBuildInputs = [ pkgs.niri ]; } ''
        mkdir niri
        cp ${files."niri/config.kdl".source} niri/config.kdl
        cp ${files."niri/theme.kdl".source} niri/theme.kdl
        cp ${files."niri/binds.kdl".source} niri/binds.kdl
        niri validate --config niri/config.kdl
        touch $out
      '';

    # niri names a cursor theme, and a theme without a shape a program asks
    # for leaves niri's built-in arrow in its place without a word, so the
    # check looks for the shapes programs ask for most in the theme it names.
    niri-cursor-has-its-shapes =
      let
        home = account graphical;
        cursor = home.home.pointerCursor;
      in
      pkgs.runCommandLocal "niri-cursor-has-its-shapes" { } ''
        grep -q 'xcursor-theme "${cursor.name}"' ${home.xdg.configFile."niri/theme.kdl".source}
        for shape in default text pointer grab grabbing crosshair not-allowed help \
          ew-resize ns-resize col-resize n-resize; do
          if [ ! -e ${cursor.package}/share/icons/${cursor.name}/cursors/$shape ]; then
            echo "${cursor.name} has no $shape cursor"
            exit 1
          fi
        done
        touch $out
      '';

    # kitty reads back its files the same way. It warns about a setting it
    # doesn't know and refuses a value it can't parse; it doesn't check that a
    # mapped action exists.
    kitty-accepts-its-configuration =
      let
        files = (account graphical).xdg.configFile;
      in
      pkgs.runCommandLocal "kitty-accepts-its-configuration" { nativeBuildInputs = [ pkgs.kitty ]; } ''
        export HOME=$PWD
        mkdir kitty
        cp ${files."kitty/kitty.conf".source} kitty/kitty.conf
        cp ${files."kitty/theme.conf".source} kitty/theme.conf
        cp ${files."kitty/keys.conf".source} kitty/keys.conf
        kitty +runpy 'from kitty.config import load_config; import sys; load_config(sys.argv[-1])' \
          kitty/kitty.conf > complaints 2>&1
        if [ -s complaints ]; then cat complaints; exit 1; fi
        touch $out
      '';

    # btop draws in the theme with the files an account gets: a theme it can't
    # find falls back to btop's own without a word, so the check looks for the
    # theme's text colour in what btop writes to its terminal.
    btop-draws-in-the-theme =
      let
        home = account workstation;
        files = home.xdg.configFile;
        foreground = lib.concatMapStringsSep ";" (byte: toString (lib.fromHexString byte)) (
          builtins.match "#(..)(..)(..)" home.jitsusama.theme.roles.text
        );
      in
      pkgs.runCommandLocal "btop-draws-in-the-theme"
        {
          nativeBuildInputs = [
            home.programs.btop.package
            pkgs.util-linux
          ];
        }
        ''
          export HOME=$PWD LANG=C.UTF-8
          mkdir -p .config/btop/themes
          cp ${files."btop/btop.conf".source} .config/btop/btop.conf
          cp ${files."btop/themes/theme.theme".source} .config/btop/themes/theme.theme
          script -q -c 'stty cols 120 rows 40; timeout 3 btop' drawn > /dev/null || true
          if ! grep -q '38;2;${foreground}m' drawn; then cat -v drawn | head -c 2000; exit 1; fi
          touch $out
        '';

    # Neovim starts with the configuration an account gets, in bamboo drawn on
    # the terminal's own background, and nothing it loads complains.
    neovim-starts-in-the-theme =
      let
        home = account workstation;
        files = home.xdg.configFile;
        # home-manager puts the plugins where Neovim looks for packages.
        plugins = home.xdg.dataFile."nvim/site/pack/hm".source;
        report = "vim.g.colors_name .. ' ' .. tostring(vim.api.nvim_get_hl(0, { name = 'Normal' }).bg)";
      in
      pkgs.runCommandLocal "neovim-starts-in-the-theme"
        {
          # The programs the account gets, git and gh among them, which
          # plugins call as they start.
          nativeBuildInputs = [ home.home.path ];
        }
        ''
          export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
          export XDG_STATE_HOME=$PWD/state XDG_CACHE_HOME=$PWD/cache
          mkdir -p config/nvim/lua data/nvim/site/pack
          ln -s ${plugins} data/nvim/site/pack/hm
          cp ${files."nvim/init.lua".source} config/nvim/init.lua
          cp ${files."nvim/lua/theme.lua".source} config/nvim/lua/theme.lua
          nvim --headless -c "lua io.stdout:write(${report})" -c 'qa!' > answer 2> complaints
          # claudecode says it has stopped as Neovim quits, which is news, not a
          # complaint.
          grep -v '\[INFO\]' complaints > errors || true
          if [ -s errors ]; then cat errors; exit 1; fi
          if [ "$(cat answer)" != "bamboo nil" ]; then cat answer; exit 1; fi
          touch $out
        '';

    # cargo builds a program with the config an account gets, and mold is the
    # linker that wrote it.
    cargo-links-with-mold =
      let
        files = (account workstation).home.file;
      in
      pkgs.runCommandCC "cargo-links-with-mold"
        {
          nativeBuildInputs = [
            pkgs.cargo
            pkgs.rustc
          ];
        }
        ''
          export HOME=$PWD CARGO_HOME=$PWD/.cargo
          mkdir -p .cargo hello/src
          cp ${files.".cargo/config.toml".source} .cargo/config.toml
          printf '[package]\nname = "hello"\nversion = "0.1.0"\nedition = "2024"\n' > hello/Cargo.toml
          echo 'fn main() {}' > hello/src/main.rs
          cargo build --offline --quiet --manifest-path hello/Cargo.toml
          readelf -p .comment hello/target/debug/hello | grep mold
          touch $out
        '';

    # CI runs the virtual machine tests by name, one to a runner, so one it
    # doesn't name would never run there.
    ci-runs-every-vm-test = pkgs.runCommandLocal "ci-runs-every-vm-test" { } ''
      for test in ${lib.concatStringsSep " " (lib.attrNames vmTests)}; do
        if ! grep -qx "          - $test" ${./.github/workflows/check.yml}; then
          echo "CI's workflow doesn't run $test."
          exit 1
        fi
      done
      touch $out
    '';
  }
  // vmTests
)
