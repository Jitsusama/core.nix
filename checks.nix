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

  # Home modules that only a Linux account can use, reached through a NixOS
  # role's home-manager.sharedModules.
  linuxOnly = [
    "bemenu"
    "fontconfig"
    "gtk"
    "quickshell"
    "signing"
    "ssh-tpm-agent"
    "swayidle"
  ];
in
checkEach "nixos" (module: bareNixos [ module ]) (removeAttrs self.nixosModules [ "disko" ])
// checkEach "darwin" (module: bareDarwin [ module ]) self.darwinModules
// checkEach "home-on-nixos" (module: bareNixos [ (inAccount module) ]) (
  removeAttrs self.homeModules [
    "signing"
    "ssh-tpm-agent"
  ]
)
// checkEach "home-on-darwin" (module: bareDarwin [ (inAccount module) ]) (
  removeAttrs self.homeModules linuxOnly
)
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
    let
      machine = bareNixos [ self.nixosModules.graphical ];
    in
    holds "onepassword-works-with-chrome" "1Password lacks its browser helper or its polkit owners." (
      machine.config.security.wrappers."1Password-BrowserSupport".group or null == "onepassword"
      && machine.config.programs._1password-gui.polkitPolicyOwners == [ "someone" ]
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
      hasYkman (bareNixos [ self.nixosModules.workstation ])
      && hasYkman (bareDarwin [ self.darwinModules.workstation ])
    );

  # ssh-tpm-agent, and the signing that uses it, need the account to reach
  # the TPM, which the tpm module gives it.
  home-on-nixos-ssh-tpm-agent = evaluates "home-on-nixos-ssh-tpm-agent" (bareNixos [
    self.nixosModules.tpm
    (inAccount self.homeModules.ssh-tpm-agent)
  ]);
  home-on-nixos-signing = evaluates "home-on-nixos-signing" (bareNixos [
    self.nixosModules.tpm
    (inAccount self.homeModules.signing)
  ]);

  # The disk module needs the machine's disk, so its check names one.
  nixos-disko = evaluates "nixos-disko" (bareNixos [
    self.nixosModules.disko
    {
      jitsusama.disk = {
        device = "/dev/disk/by-id/nvme-example";
        swapSize = "16G";
      };
    }
  ]);

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
        files = (account (bareNixos [ self.nixosModules.graphical ])).xdg.configFile;
      in
      pkgs.runCommandLocal "niri-accepts-its-configuration" { nativeBuildInputs = [ pkgs.niri ]; } ''
        mkdir niri
        cp ${files."niri/config.kdl".source} niri/config.kdl
        cp ${files."niri/theme.kdl".source} niri/theme.kdl
        niri validate --config niri/config.kdl
        touch $out
      '';

    # niri names a cursor theme, and a theme without a shape a program asks
    # for leaves niri's built-in arrow in its place without a word, so the
    # check looks for the shapes programs ask for most in the theme it names.
    niri-cursor-has-its-shapes =
      let
        home = account (bareNixos [ self.nixosModules.graphical ]);
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
        files = (account (bareNixos [ self.nixosModules.graphical ])).xdg.configFile;
      in
      pkgs.runCommandLocal "kitty-accepts-its-configuration" { nativeBuildInputs = [ pkgs.kitty ]; } ''
        export HOME=$PWD
        mkdir kitty
        cp ${files."kitty/kitty.conf".source} kitty/kitty.conf
        cp ${files."kitty/theme.conf".source} kitty/theme.conf
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
        home = account (bareNixos [ self.nixosModules.workstation ]);
        files = home.xdg.configFile;
        foreground = lib.concatMapStringsSep ";" (byte: toString (lib.fromHexString byte)) (
          builtins.match "#(..)(..)(..)" home.jitsusama.theme.colors.foreground
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
        home = account (bareNixos [ self.nixosModules.workstation ]);
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
        files = (account (bareNixos [ self.nixosModules.workstation ])).home.file;
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

    # Boots a virtual machine through the whole install, since the disk layout
    # and Secure Boot decide whether a machine starts at all.
    secure-boot-installs = import ./tests/secure-boot.nix { inherit self pkgs; };
    ssh-tpm-agent-signs = import ./tests/ssh-tpm-agent.nix { inherit self pkgs; };
    commits-are-signed = import ./tests/signing.nix { inherit self pkgs; };
    keyring-opens-without-asking = import ./tests/gnome-keyring.nix { inherit self pkgs; };
    speakers-are-tuned = import ./tests/speakers.nix { inherit pkgs; };
    desktop-works = import ./tests/desktop.nix { inherit self pkgs; };
  }
)
