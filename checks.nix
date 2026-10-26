# Everything `nix flake check` runs, from the bottom of the pyramid up: the
# promises modules and roles make, read from evaluated machines; the example
# machines, which use core only through its outputs and so evaluate every
# module there is; the real programs reading the files an account gets; and a
# few virtual machines, each showing what nothing cheaper can. Each check has
# to give a signal the ones below it don't, and docs/testing.md says which.
#
# Each check sits in the checks.nix of the folder whose promise it holds, a
# module's or a hardware model's, given the helpers below by name. This file
# keeps the helpers, the virtual machines and the checks about the
# repository as a whole, and gathers every folder's.
{
  self,
  pkgs,
  nixpkgs,
  nix-darwin,
  rust-overlay,
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

  # Puts an account's Neovim where Neovim looks for it, under $PWD: every
  # file the account has in ~/.config/nvim, and the plugins.
  neovimAt =
    home:
    let
      files = home.xdg.configFile;
      names = lib.filter (lib.hasPrefix "nvim/") (lib.attrNames files);
    in
    ''
      export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
      export XDG_STATE_HOME=$PWD/state XDG_CACHE_HOME=$PWD/cache
      mkdir -p data/nvim/site/pack
      ln -s ${home.xdg.dataFile."nvim/site/pack/hm".source} data/nvim/site/pack/hm
      ${lib.concatMapStrings (name: ''
        mkdir -p config/${dirOf name}
        cp ${files.${name}.source} config/${name}
      '') names}
    '';

  # Runs a test that times itself in Qt's engine, in a fresh process each
  # time, until one is fast enough. How fast the engine runs the same code
  # differs from one process to the next by as much as half, while a
  # slower search slows every process, so the fastest of five is what's
  # judged (tests/qml-in-time.sh).
  timedQml = file: ''
    ${shellFolderLint}
    ${lib.getExe pkgs.bash} ${./tests/qml-in-time.sh} ${pkgs.qt6.qtdeclarative} ${file}
  '';

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
  # minutes; the kernel itself takes far longer than a check should
  # (tests/kernel-settings-hold.sh).
  kernelSettingsHold =
    name: kernel:
    let
      # The kernel's own, as package.nix gave them, so a machine's theme
      # colours are held as well as the settings every kernel shares.
      settings = lib.mapAttrs (_option: setting: setting.content or setting) kernel.structuredExtraConfig;
      # A setting's line in the configuration, its option, and whether it's
      # optional, by tabs.
      row =
        option: setting:
        lib.concatStringsSep "\t" [
          (kernelConfigLine option setting)
          option
          (if setting.optional or false then "optional" else "required")
        ];
      rows = pkgs.writeText "${name}-settings" (lib.concatLines (lib.mapAttrsToList row settings));
    in
    pkgs.runCommandLocal name { } ''
      ${lib.getExe pkgs.bash} ${./tests/kernel-settings-hold.sh} ${kernel.configfile} ${rows}
      touch $out
    '';

  # The machines most checks read, each evaluated once and shared, since an
  # evaluation is most of what a check costs.
  workstation = bareNixos [ self.nixosModules.workstation ];
  graphical = bareNixos [ self.nixosModules.graphical ];
  macWorkstation = bareDarwin [ self.darwinModules.workstation ];

  # What clippy holds Rust to, and its settings, as a repository built on core
  # gets them too; modules/clippy/clippy.nix says why each lint is there.
  clippy = pkgs.callPackage ./modules/clippy/clippy.nix { };
  clippyLints = clippy.lints;
  clippyConfig = clippy.config;

  # Quickshell's own type files, which qmllint reads the shell against, with
  # two of their faults corrected: the blur module doesn't say it depends on
  # Quickshell, whose Region it names, and PanelWindow is marked uncreatable
  # because Quickshell types it as the interface it makes a real window
  # behind. The launcher's maths sits beside them, as the machine's
  # Quickshell finds it (modules/quickshell/nixos.nix).
  quickshellTypes = pkgs.runCommandLocal "quickshell-types" { } ''
    mkdir -p $out/lib/qt-6/qml
    cp -r ${pkgs.quickshell}/lib/qt-6/qml/Quickshell $out/lib/qt-6/qml/
    cp -r ${pkgs.callPackage ./modules/quickshell/maths/package.nix { }}/lib/qt-6/qml/Maths \
      $out/lib/qt-6/qml/
    chmod -R u+w $out
    echo 'depends Quickshell' >> $out/lib/qt-6/qml/Quickshell/Wayland/_BackgroundEffect/qmldir
    substituteInPlace $out/lib/qt-6/qml/Quickshell/_Window/quickshell-window.qmltypes \
      --replace-fail 'exports: ["Quickshell._Window/PanelWindow 0.0"]
            isCreatable: false' 'exports: ["Quickshell._Window/PanelWindow 0.0"]'
  '';

  # Lints the shell an offscreen check has laid out in its build folder, its
  # own cases and stand-ins included, as the account's is linted
  # (tests/shell-folder-lint.sh).
  shellFolderLint = lib.concatStringsSep " " [
    (lib.getExe (pkgs.callPackage ./tests/shell-folder-lint.nix { }))
    "${quickshellTypes}/lib/qt-6/qml"
    "${pkgs.kdePackages.qtdeclarative}/lib/qt-6/qml"
  ];

  # A Rust package with clippy in place of its build, its tests and examples
  # read too, held to clippyLints, and nothing installed.
  passesClippy = name: package: clippy.passes { inherit name package; };

  # The packages a VM test's host runs it with, the virtiofsd its VMs share
  # folders through started by without-syncfs (modules/nix-daemon), so a
  # guest's sync of a share never commits the host's whole filesystem,
  # whether the daemon runs the test or a person runs its driver. Only the
  # host's set changes: the guests keep their own packages, which libvirt's
  # virtiofsd comes from.
  vmHostPkgs = pkgs // {
    virtiofsd = pkgs.writeShellScriptBin "virtiofsd" ''
      exec ${
        lib.getExe (pkgs.callPackage ./modules/nix-daemon/without-syncfs/package.nix { })
      } ${lib.getExe pkgs.virtiofsd} "$@"
    '';
    testers = pkgs.testers // {
      runNixOSTest =
        test:
        pkgs.testers.runNixOSTest (
          { config, ... }:
          {
            imports = [ test ];
            hostPkgs = lib.mkForce vmHostPkgs;
            # Each machine's qemu, whichever the test chose, waits for the
            # virtiofsd sockets it connects to before starting.
            nodeDefaults.virtualisation.qemu.package = lib.mkForce (qemuAfterItsSockets config.qemu.package);
          }
        );
    };
  };
  qemuAfterItsSockets = pkgs.callPackage ./tests/lib/qemu-after-its-sockets.nix { };

  # The virtual machine tests. Each has to show something nothing cheaper can.
  vmTestsOn = pkgs: {
    # The disk layout and Secure Boot decide whether a machine starts at all.
    vm-secure-boot-installs = import ./tests/vm/guide/secure-boot.nix { inherit self pkgs; };
    # The guide's install is a stick, a disk, FIDO2 keys, the firmware and
    # a TPM together, driven by its keys as a person drives it.
    vm-guide-installs = import ./tests/vm/guide/guide-install.nix { inherit self pkgs; };
    # Writing a stick on a host is UDisks2, polkit and a USB disk together,
    # as the person rather than root, and a bundle sealed to a FIDO2 key
    # opens only where the plugin, the key and its PIN meet.
    vm-guide-writes-a-stick = import ./tests/vm/guide/guide-writer.nix { inherit self pkgs; };
    # Recovering is a stick, a laid-out disk and its subvolumes together,
    # with nothing of the machine's on the stick.
    vm-guide-recovers = import ./tests/vm/guide/guide-recover.nix { inherit self pkgs; };
    vm-commits-are-signed = import ./tests/signing.nix { inherit self pkgs; };
    # A passkey is the TPM, uhid, udev's seat rules and the shell's askpass
    # together, and only a running machine has them all.
    vm-passkeys-come-from-the-tpm = import ./tests/linux-id.nix { inherit self pkgs; };
    vm-keyring-opens-without-asking = import ./tests/gnome-keyring.nix { inherit self pkgs; };
    vm-speakers-are-tuned = import ./tests/speakers.nix { inherit pkgs; };
    vm-pebble-speakers-are-tuned = import ./tests/pebble-x-plus.nix { inherit pkgs; };
    vm-mv7-voice-is-levelled = import ./tests/shure-mv7-voice.nix { inherit pkgs; };
    # Which input WirePlumber picks comes from the rank it gives each
    # microphone as udev reports its card, so only a running machine shows it.
    vm-webcam-microphones-yield = import ./tests/microphones.nix { inherit pkgs; };
    # A light plugged back in is uhid, hidraw, udev's nodes in /dev and the
    # lights' watches together, which only a running machine has.
    vm-the-desk-lights-come-back = import ./tests/desk-lights.nix { inherit self pkgs; };
    # A card plugged in and pulled out is the kernel's uevents and its USB
    # audio driver together, which only a running machine has.
    vm-desk-facts-follow-a-replug = import ./tests/desk-facts.nix { inherit self pkgs; };
    # The desktop, in areas of a minute or two each on one machine
    # (tests/desktop.nix).
    vm-desktop-starts-clean = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "starts-clean";
    };
    vm-desktop-launcher-goes-places = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-goes-places";
    };
    vm-desktop-launcher-answers = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-answers";
    };
    vm-desktop-opens-and-sets = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "opens-and-sets";
    };
    vm-desktop-says-what-happens = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "says-what-happens";
    };
    vm-desktop-captures-and-locks = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "captures-and-locks";
    };
    # Joining Wi-Fi is the kernel, wifid, wpa_supplicant and networkd
    # together, and what it leaves of other programs' routing only shows
    # once it has.
    vm-wifi-joins = import ./tests/wifi.nix { inherit self pkgs; };
    vm-disk-guard-keeps-the-disk = import ./tests/disk-guard.nix { inherit self pkgs; };
    # What an account's services make of the network coming late, changing
    # and networkd restarting shows only with both managers running.
    vm-network-revives-units = import ./tests/network-revive.nix { inherit self pkgs; };
    # Which server timesyncd asks first shows only on a network that names
    # one of its own.
    vm-clock-keeps-time = import ./tests/clock.nix { inherit self pkgs; };
    # The greeter is all that stands between a logout and the desktop.
    vm-greeter-signs-in = import ./tests/greeter.nix { inherit self pkgs; };
    # Whether the shell's surfaces come back after a screen goes shows only
    # with niri on the screen itself, not nested in Cage.
    vm-screens-come-and-go = import ./tests/screens.nix { inherit self pkgs; };
    vm-boot-asks-in-the-theme = import ./tests/plymouth.nix { inherit self pkgs; };
    # Whether the ask shows whole on screens of different sizes shows only
    # with Plymouth drawing on two of them at once.
    vm-boot-asks-on-every-screen = import ./tests/plymouth-screens.nix { inherit self pkgs; };
    # Whether libvirt's daemon stops in time shows only with the daemon
    # running and its threads short of CPU, as at a busy shutdown.
    vm-libvirt-stops-quickly = import ./tests/libvirt.nix { inherit self pkgs; };
    # Whether the IR camera opens only while it's read, and hands each reader
    # its frames and none of the last one's, shows only with cameras there.
    vm-ir-camera-opens-for-readers = import ./tests/ir-camera.nix { inherit self pkgs; };
    # Whether the programs holding a loopback camera are named shows only with
    # a loopback there and another account's programs holding it.
    vm-loopback-says-who-holds-it = import ./tests/loopback-holders.nix { inherit self pkgs; };
    # Whether the swap comes down while udev probes its device shows only with
    # the device there and held open.
    vm-swap-comes-down-while-probed = import ./tests/zram.nix { inherit self pkgs; };
    # Whether systemd-oomd ends the work that filled swap before the kernel's
    # OOM killer picks a program shows only with swap filling for real.
    vm-full-swap-ends-what-filled-it = import ./tests/full-swap.nix { inherit self pkgs; };
    # Whether the kernel took a slice's disk latency shows only with the
    # account's manager starting it on a real drive.
    vm-apps-come-first = import ./tests/slices.nix { inherit self pkgs; };
    # Which of CAKE's tins a slice's traffic reaches shows only with the
    # account's manager making the slices and packets crossing a real link.
    vm-the-network-keeps-its-order = import ./tests/shaper.nix { inherit self pkgs; };
    # Whether the shaper finds a connection's rate, and keeps a ping near its
    # own delay under a full upload, shows only with a real queue filling.
    vm-the-connection-stays-quick = import ./tests/shaper-rate.nix { inherit self pkgs; };
    # Whether a check's VM goes ahead of the builds shows only with a build
    # user's process in the daemon's own cgroup and the service's real rights.
    vm-checks-go-first = import ./tests/vm-first.nix { inherit self pkgs; };
    # Whether a reboot gets past what won't stop shows only by rebooting, with
    # a watchdog there to reset the machine.
    vm-reboot-cannot-hang = import ./tests/shutdown.nix { inherit self pkgs; };
    # Whether dictation's chord is heard let go, and nothing else of the
    # keys, shows only with a keyboard there and an account outside its group.
    vm-dictation-hears-the-chord-let-go = import ./tests/dictation-chord.nix { inherit self pkgs; };
    # When dictation says it's listening shows only with a sound server there,
    # with a microphone and without.
    vm-dictation-listens-once-it-hears = import ./tests/dictation-listening.nix {
      inherit self pkgs;
    };
    # What logind makes of a shut lid shows only with logind watching one.
    vm-lid-stays-at-the-desk = import ./tests/lid.nix { inherit self pkgs; };
    # Whether udev turns a root hub's wakeup on shows only with a USB
    # controller and udev there.
    vm-plugging-in-wakes-it = import ./tests/usb-wake.nix { inherit self pkgs; };
    # Whether polkit lets an account switch the machine shows only with
    # polkit, systemd and a system to switch to.
    vm-admins-switch-unasked = import ./tests/switch-system.nix { inherit self pkgs; };
    # Whether polkit lets an administrator tune the work slices shows only
    # with polkit and systemd.
    vm-admins-tune-the-work-slices = import ./tests/work-slices.nix { inherit self pkgs; };
    # Whether a niri's kernel stacks are traced for its account and nobody
    # else shows only with polkit, bpftrace and a kernel to trace.
    vm-niri-is-traced-by-its-own = import ./tests/niri-off-cpu.nix { inherit self pkgs; };
  };
  vmTests = vmTestsOn vmHostPkgs;

  # Checks for Linux alone, which a Mac neither builds nor evaluates.
  onLinux = lib.optionalAttrs (pkgs.stdenv.hostPlatform.system == "x86_64-linux");

  # What every folder's checks.nix is given, of which each takes what it uses.
  given = {
    inherit
      self
      pkgs
      lib
      nixpkgs
      nix-darwin
      rust-overlay
      formatting
      bareNixos
      bareDarwin
      evaluates
      same
      holds
      failure
      account
      neovimAt
      timedQml
      kernelConfigLine
      kernelSettingsHold
      workstation
      graphical
      macWorkstation
      clippy
      clippyLints
      clippyConfig
      quickshellTypes
      shellFolderLint
      passesClippy
      onLinux
      ;
  };

  # What each check proves (check-descriptions.nix), each a sentence however
  # its lines are wrapped.
  descriptions = lib.mapAttrs (
    _: text: lib.concatStringsSep " " (lib.splitString "\n" (lib.trim text))
  ) (import ./check-descriptions.nix);

  # Every jitsusama.* option a Linux machine can set, NixOS's and
  # home-manager's, read from the two Linux examples, which between them
  # import every module Linux has.
  optionNames =
    let
      machines =
        map
          (
            example:
            (import example {
              core = self;
              inherit nixpkgs;
            }).options
          )
          [
            ./examples/nixos.nix
            ./examples/laptop.nix
          ];
      spelled =
        options:
        map (option: lib.showOption (lib.remove "<name>" option.loc)) (
          lib.collect lib.isOption (options.jitsusama or { })
        );
      home = options: options.home-manager.users.type.getSubOptions [ ];
    in
    lib.unique (lib.concatMap (options: spelled options ++ spelled (home options)) machines);

  # Gives every check what it proves as its meta.description, so a query of the
  # checks lists them, and holds that every check says it and every description
  # names a check. Only Linux has every check, so only it holds that.
  describe =
    checks:
    let
      named = lib.attrNames checks ++ [
        "every-check-says-what-it-proves"
        "no-name-says-a-replaced-word"
      ];
      undescribed = lib.subtractLists (lib.attrNames descriptions) named;
      unchecked = lib.subtractLists named (lib.attrNames descriptions);
      wrong =
        lib.optional (undescribed != [ ])
          "these checks say nothing in check-descriptions.nix of what they prove: ${lib.concatStringsSep ", " undescribed}"
        ++
          lib.optional (unchecked != [ ])
            "these descriptions in check-descriptions.nix name no check: ${lib.concatStringsSep ", " unchecked}";
      all =
        checks
        // onLinux {
          every-check-says-what-it-proves =
            holds "every-check-says-what-it-proves" "Of the checks, ${lib.concatStringsSep "; and " wrong}."
              (wrong == [ ]);

          # Every check's name and every jitsusama.* option's says the
          # glossary's words, never a name it replaces, held as a repository
          # built on core holds its own (tests/names.nix).
          no-name-says-a-replaced-word = self.lib.namesSayTheGlossary pkgs (
            { names = named ++ optionNames; } // import ./tests/names-kept.nix
          );
        };
    in
    lib.mapAttrs (
      name: check: lib.addMetaAttrs { description = descriptions.${name} or null; } check
    ) all;

  # Every folder's checks, beside the repository's own, each told what it
  # proves. A name two of them give stops the evaluation rather than one check
  # hiding the other.
  gather =
    sets:
    describe (
      lib.foldl' (
        gathered: these:
        let
          clashes = lib.intersectLists (lib.attrNames gathered) (lib.attrNames these);
        in
        if clashes == [ ] then
          gathered // these
        else
          throw "More than one file gives the checks ${lib.concatStringsSep ", " clashes}"
      ) { } sets
    );
in
gather (
  [
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
          # The launcher's colour answers, and the colours they're tested on, name
          # colours because colours are what's asked; neither is a look.
          # ripgrep answers 0 for a match, 1 for none and 2 when it can't
          # search, and only none passes.
          status=0
          rg --line-number --no-heading --hidden --glob '!modules/theme/**' \
            --glob '!modules/quickshell/launcher/Colours.js' --glob '!tests/colour-cases.js' \
            -e ${lib.escapeShellArg pattern} modules roles hardware tests examples flake.nix \
            || status=$?
          if [ $status = 0 ]; then
            echo "The lines above name a colour or a face; name the theme's role or font instead."
            exit 1
          fi
          [ $status = 1 ]
          touch $out
        '';

      # Every surface draws its marks from the theme's table (modules/theme/
      # glyphs.nix) by name, so no module but the theme's spells one of the
      # table's code points itself: a surface that did would keep its mark when
      # the table changes, and could drift from the mark the same idea wears
      # elsewhere. A code point is caught as a script writes one, 0xf023 or an
      # escape; tests spell what they expect to see, which is their job, so no
      # tests folder is read, at the top or beside its module.
      nothing-spells-a-mark =
        pkgs.runCommandLocal "nothing-spells-a-mark" { nativeBuildInputs = [ pkgs.ripgrep ]; }
          ''
            cd ${./.}
            codes=$(rg --only-matching --no-filename --replace '$1' 'nerd "([0-9a-fA-F]+)"' \
              modules/theme/glyphs.nix | sort -u | paste -sd '|')
            [ -n "$codes" ]
            # ripgrep answers 0 for a match, 1 for none and 2 when it can't
            # search, and only none passes.
            status=0
            rg --line-number --no-heading --hidden --ignore-case --glob '!modules/theme/**' \
              --glob '!**/tests/**' \
              -e "(0x|\\\\u\\{?|\\\\U000)0*($codes)\\b" modules roles hardware examples flake.nix \
              || status=$?
            if [ $status = 0 ]; then
              echo "The lines above spell a mark the theme names; use the theme's mark instead."
              exit 1
            fi
            [ $status = 1 ]
            touch $out
          '';

      # Quickshell's JavaScript lacks some of the language's newer methods, which
      # qmllint lets through and the shell fails on only as it runs, so the
      # shell's code calls none of them.
      the-shell-speaks-its-javascript =
        pkgs.runCommandLocal "the-shell-speaks-its-javascript" { nativeBuildInputs = [ pkgs.ripgrep ]; }
          ''
            cd ${./.}
            # ripgrep answers 0 for a match, 1 for none and 2 when it can't
            # search, and only none passes.
            status=0
            rg --line-number --no-heading --glob '*.qml' --glob '*.js' \
              -e '\.(flatMap|flat|findLast|findLastIndex|toSorted|toReversed|toSpliced)\(' \
              -e '\b(structuredClone|Object\.groupBy|Object\.hasOwn)\(' modules/quickshell || status=$?
            if [ $status = 0 ]; then
              echo "The lines above call a method Quickshell's JavaScript doesn't have."
              exit 1
            fi
            [ $status = 1 ]
            touch $out
          '';

      # changed-checks names a check by how its derivation moved, so what it
      # builds is what a change touched: its tests run as it builds, the
      # whole program among them, against stand-ins for git and nix
      # (tools/changed-checks).
      changed-checks-names-what-moved = pkgs.callPackage ./tools/changed-checks/package.nix { };

      # clippy finds nothing in changed-checks, its tests included.
      changed-checks-passes-clippy = passesClippy "changed-checks-passes-clippy" (
        pkgs.callPackage ./tools/changed-checks/package.nix { }
      );

      # run-subtests keeps a test script's boot and helpers and only the subtests
      # named, and runs a pytest one whole with its tests picked, so what it runs
      # is what was asked for (tools/run-subtests/run-subtests-test.py, against
      # stand-ins for nix and a check's driver).
      run-subtests-keeps-what-is-named =
        let
          run-subtests = pkgs.callPackage ./tools/run-subtests/run-subtests.nix { };
          python = pkgs.python3.withPackages (python: [ python.pytest ]);
        in
        pkgs.runCommandLocal "run-subtests-keeps-what-is-named" { } ''
          RUN_SUBTESTS=${lib.getExe run-subtests} PYTHONDONTWRITEBYTECODE=1 \
            ${python}/bin/python -m pytest -p no:cacheprovider --import-mode=importlib \
            --quiet ${./tools/run-subtests/run-subtests-test.py}
          touch $out
        '';

      # What VM tests share (tests/lib), proven against stand-ins for the
      # driver and its machines, which record what they're handed. Named
      # without vm- so the machine queue doesn't take these milliseconds for
      # a VM test.
      what-vm-tests-share-does-what-it-says =
        let
          python = pkgs.python3.withPackages (python: [ python.pytest ]);
        in
        pkgs.runCommandLocal "what-vm-tests-share-does-what-it-says" { } ''
          cd ${./tests/lib}
          PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 ${python}/bin/python -m pytest \
            -p no:cacheprovider --import-mode=importlib --quiet \
            vm-helpers-test.py vm-pytest-test.py
          touch $out
        '';

      # The qemu VM tests run, put in front of a stand-in qemu that says how
      # it was run.
      qemu-starts-after-its-sockets =
        let
          python = pkgs.python3.withPackages (python: [ python.pytest ]);
          standIn =
            pkgs.runCommandLocal "qemu-stand-in" { } ''
              mkdir -p $out/bin
              printf '#!%s\nprintf "%%s\\n" "$@" > "$QEMU_RAN"\n' ${pkgs.runtimeShell} \
                > $out/bin/qemu-system-x86_64
              printf '#!%s\n' ${pkgs.runtimeShell} > $out/bin/qemu-img
              chmod +x $out/bin/*
            ''
            // {
              inherit (pkgs) stdenv;
            };
        in
        pkgs.runCommandLocal "qemu-starts-after-its-sockets" { } ''
          QEMU=${qemuAfterItsSockets standIn} PYTHONDONTWRITEBYTECODE=1 \
            ${python}/bin/python -m pytest -p no:cacheprovider --import-mode=importlib \
            --quiet ${./tests/lib/qemu-after-its-sockets-test.py}
          touch $out
        '';

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

      # Every folder says what goes in it, so a reader reaches any file from the
      # map by names alone: in a README, the opening comment of its entry file
      # or of a file beside it of its name, a line of its parent's README
      # opening with its name, or a row of AGENTS.md's map. The
      # folders below are still waiting for their note, and the check fails
      # once one has it or is gone, so the list only shrinks.
      every-folder-says-what-it-holds =
        let
          waiting = [
            # The desk keyboard's layout, which firmware.nix describes. Its
            # folder's hash is the board's serial, so a note in it would change
            # the firmware and need a reflash; it takes one with the next
            # layout change.
            "modules/ergodox/keymap"
          ];
        in
        pkgs.runCommandLocal "every-folder-says-what-it-holds" { } ''
          ${lib.getExe pkgs.bash} ${./tests/every-folder-says-what-it-holds.sh} ${./.} ${pkgs.writeText "waiting" (lib.concatLines waiting)}
          touch $out
        '';

      # A doc names only what's there, so a move or a rename that leaves a guide
      # pointing at nothing fails here rather than misleading the next reader:
      # every relative link, and every backticked path from a top folder. Those
      # below name something deliberately absent, and fail once they're gone.
      docs-name-what-exists = pkgs.callPackage ./tests/docs-name-what-exists.nix { } {
        src = ./.;
        exempt = [
          # A module a reader would add, as an example.
          "docs/modules.md modules/jq/checks.nix"
          # A file in the kernel's own source tree.
          "docs/hardware.md tools/power/x86/turbostat/Makefile"
        ];
      };

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

      # The glossary gives each thing one word and each word one thing within
      # its context, and writes no name it replaces (tests/glossary.py),
      # printing each word to settle.
      the-glossary-names-each-thing-once =
        pkgs.runCommandLocal "the-glossary-names-each-thing-once" { }
          ''
            ${lib.getExe pkgs.python3} ${./tests/glossary.py} ${./docs/glossary.md}
            touch $out
          '';
    }
    # shell-preview shows the shell nested in a Wayland session, which only
    # Linux has.
    (onLinux {
      # shell-preview links each built file the working copy holds back to it,
      # so an edit shows without a build, and copies what the build made, and
      # ends the nested desktop it started however it's ended: its tests run
      # as it builds, the whole program among them, against stand-ins for git,
      # nix and the desktop (tools/shell-preview).
      shell-preview-follows-the-working-copy = pkgs.callPackage ./tools/shell-preview/package.nix { };

      # clippy finds nothing in shell-preview, its tests included.
      shell-preview-passes-clippy = passesClippy "shell-preview-passes-clippy" (
        pkgs.callPackage ./tools/shell-preview/package.nix { }
      );
    })
    (onLinux vmTests)
  ]
  ++ map (file: import file given) [
    ./hardware/dell-xps-14-da14260/checks.nix
    ./hardware/microsoft-surface-laptop-4-1958/checks.nix
    ./modules/backlight/checks.nix
    ./modules/bat/checks.nix
    ./modules/bluetooth-links/checks.nix
    ./modules/btop/checks.nix
    ./modules/btrfs/checks.nix
    ./modules/cargo/checks.nix
    ./modules/claude/checks.nix
    ./modules/clippy/checks.nix
    ./modules/console/checks.nix
    ./modules/creative-pebble-x-plus/checks.nix
    ./modules/ddcutil/checks.nix
    ./modules/desk-facts/checks.nix
    ./modules/device-check/checks.nix
    ./modules/dictation/checks.nix
    ./modules/disk-guard/checks.nix
    ./modules/disko/checks.nix
    ./modules/eero/checks.nix
    ./modules/ergodox/checks.nix
    ./modules/firmware-settings/checks.nix
    ./modules/fonts/checks.nix
    ./modules/gimp/checks.nix
    ./modules/git/checks.nix
    ./modules/git-engine/checks.nix
    ./modules/gtk/checks.nix
    ./modules/guide/checks.nix
    ./modules/inkscape/checks.nix
    ./modules/inlyne/checks.nix
    ./modules/ir-camera/checks.nix
    ./modules/kernel/checks.nix
    ./modules/keys/checks.nix
    ./modules/kitty/checks.nix
    ./modules/kmscon/checks.nix
    ./modules/lag-watch/checks.nix
    ./modules/load-proof/checks.nix
    ./modules/lanzaboote/checks.nix
    ./modules/libvirt/checks.nix
    ./modules/lid/checks.nix
    ./modules/linux-id/checks.nix
    ./modules/litra/checks.nix
    ./modules/memory-modules/checks.nix
    ./modules/mpv/checks.nix
    ./modules/neovim/checks.nix
    ./modules/network-revive/checks.nix
    ./modules/nftables/checks.nix
    ./modules/niri/checks.nix
    ./modules/nix-daemon/checks.nix
    ./modules/onepassword/checks.nix
    ./modules/open-with/checks.nix
    ./modules/panel/checks.nix
    ./modules/pi/checks.nix
    ./modules/pipewire/checks.nix
    ./modules/plymouth/checks.nix
    ./modules/pnpm/checks.nix
    ./modules/power-profiles-daemon/checks.nix
    ./modules/prompt/checks.nix
    ./modules/qt/checks.nix
    ./modules/quickshell/checks.nix
    ./modules/session-backup/checks.nix
    ./modules/shaper/checks.nix
    ./modules/signing/checks.nix
    ./modules/slices/checks.nix
    ./modules/spend-watch/checks.nix
    ./modules/sudo/checks.nix
    ./modules/spotify/checks.nix
    ./modules/ssh-tpm-agent/checks.nix
    ./modules/swayidle/checks.nix
    ./modules/swayimg/checks.nix
    ./modules/systemd/checks.nix
    ./modules/systemd-networkd/checks.nix
    ./modules/tcp/checks.nix
    ./modules/theme/checks.nix
    ./modules/touchpad/checks.nix
    ./modules/tuple/checks.nix
    ./modules/v4l2loopback/checks.nix
    ./modules/wifid/checks.nix
    ./modules/yubikey/checks.nix
    ./modules/zathura/checks.nix
    ./modules/zsh/checks.nix
  ]
)
