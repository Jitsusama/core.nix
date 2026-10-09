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

  # The files a machine imports, and the examples that show how. Documentation
  # may name an employer; these may not.
  imported = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./modules
      ./roles
      ./hardware
      ./examples
    ];
  };

  # Home modules that only a Linux account can use, reached through a NixOS
  # role's home-manager.sharedModules.
  linuxOnly = [
    "bemenu"
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
