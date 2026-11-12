# Everything `nix flake check` runs, from the bottom of the pyramid up: the
# promises modules and roles make, read from evaluated machines; the example
# machines, which use core only through its outputs and so evaluate every
# module there is; the real programs reading the files an account gets; and a
# few virtual machines, each showing what nothing cheaper can. Each check has
# to give a signal the ones below it don't, and docs/testing.md says which.
#
# Each check sits in the checks.nix of the folder whose promise it holds, a
# module's, a hardware model's, a tool's, a VM area's or the repository's own
# in tests/repository/ and tests/rules/, given the helpers below by name. This
# file keeps the helpers, the virtual machines not yet in an area of
# tests/vm/, and the checks of the roles, and gathers every folder's.
{
  self,
  pkgs,
  nixpkgs,
  nix-darwin,
  rust-overlay,
  formatting,
  formatterExcludes,
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
  # three of their faults corrected: the blur module doesn't say it depends
  # on Quickshell, whose Region it names; PanelWindow is marked uncreatable
  # because Quickshell types it as the interface it makes a real window
  # behind; and Qt 6.12's type registrar writes into a module's file an
  # opaque stand-in, a value with no members, for each type the module
  # names but doesn't declare, which qmllint reads in place of the real
  # type. A window's screen and margins, an ObjectModel's values and a
  # notice's actions had nothing to read, and a screen could never be null,
  # so those stand-ins are taken out, as Qt 6.11 wrote the files: qmllint
  # finds the types Quickshell's core declares, and the actions as a list of
  # the module's own NotificationAction. The launcher's maths and key times
  # sit beside them, as the machine's Quickshell finds them
  # (modules/quickshell/nixos.nix). They're read from the machine's own
  # Quickshell, its patches laid on, so a type a patch adds, as
  # FramePresentation, is known.
  quickshellTypes = pkgs.runCommandLocal "quickshell-types" { } ''
    mkdir -p $out/lib/qt-6/qml
    cp -r ${graphical.pkgs.quickshell}/lib/qt-6/qml/Quickshell $out/lib/qt-6/qml/
    cp -r ${pkgs.callPackage ./modules/quickshell/maths/package.nix { }}/lib/qt-6/qml/Maths \
      $out/lib/qt-6/qml/
    cp -r ${pkgs.callPackage ./modules/quickshell/key-times/package.nix { }}/lib/qt-6/qml/KeyTimes \
      $out/lib/qt-6/qml/
    chmod -R u+w $out
    echo 'depends Quickshell' >> $out/lib/qt-6/qml/Quickshell/Wayland/_BackgroundEffect/qmldir
    echo 'depends Quickshell' >> $out/lib/qt-6/qml/Quickshell/Wayland/_Presentation/qmldir
    substituteInPlace $out/lib/qt-6/qml/Quickshell/_Window/quickshell-window.qmltypes \
      --replace-fail 'exports: ["Quickshell._Window/PanelWindow 0.0"]
            isCreatable: false' 'exports: ["Quickshell._Window/PanelWindow 0.0"]'
    # Takes a module's stand-in for a type out of its file.
    unstand() {
      substituteInPlace "$out/lib/qt-6/qml/Quickshell/$1" --replace-fail \
        "    Component { isTypeOpaque: true; name: \"$2\"; accessSemantics: \"value\" }" ""
    }
    unstand _Window/quickshell-window.qmltypes Margins
    unstand _Window/quickshell-window.qmltypes QuickshellScreenInfo
    unstand Wayland/quickshell-wayland.qmltypes QuickshellScreenInfo
    unstand Services/Mpris/quickshell-service-mpris.qmltypes UntypedObjectModel
    unstand Services/Pipewire/quickshell-service-pipewire.qmltypes UntypedObjectModel
    unstand Services/UPower/quickshell-service-upower.qmltypes UntypedObjectModel
    unstand Services/Notifications/quickshell-service-notifications.qmltypes UntypedObjectModel
    substituteInPlace \
      $out/lib/qt-6/qml/Quickshell/Services/Notifications/quickshell-service-notifications.qmltypes \
      --replace-fail '    Component {
            isTypeOpaque: true
            name: "QList<qs::service::notifications::NotificationAction*>"
            accessSemantics: "value"
        }' ""
  '';

  # Lints the shell an offscreen check has laid out in its build folder, its
  # own cases and stand-ins included, as the account's is linted
  # (tests/lib/shell-folder-lint.sh).
  shellFolderLint = lib.concatStringsSep " " [
    (lib.getExe (pkgs.callPackage ./tests/lib/shell-folder-lint.nix { }))
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
    # Only Chrome, started, says whether it installed what the preferences
    # file names, and writes a session file as it browses.
    vm-chrome-says-its-open-tabs = import ./tests/chrome-extension.nix {
      inherit self pkgs;
    };
    # The desktop, in areas of a minute or two each on one machine
    # (tests/desktop.nix).
    vm-desktop-starts-clean = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "starts-clean";
    };
    vm-desktop-shows-then-rests = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "shows-then-rests";
    };
    vm-desktop-launcher-goes-places = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-goes-places";
    };
    vm-desktop-launcher-finds-sessions = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-finds-sessions";
    };
    vm-desktop-moves-keep-their-window = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "moves-keep-their-window";
    };
    vm-desktop-reopens-under-orchestrators = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "reopens-under-orchestrators";
    };
    vm-desktop-launcher-waits-on-joel = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-waits-on-joel";
    };
    vm-desktop-launcher-holds-meetings = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-holds-meetings";
    };
    vm-desktop-launcher-reads-mail = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-reads-mail";
    };
    vm-desktop-launcher-moves-windows = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-moves-windows";
    };
    vm-desktop-launcher-draws-still = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-draws-still";
    };
    vm-desktop-launcher-answers = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-answers";
    };
    vm-desktop-launcher-finds-by-name = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-finds-by-name";
    };
    vm-desktop-keeps-copies = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "keeps-copies";
    };
    vm-desktop-opens-files = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "opens-files";
    };
    vm-desktop-opens-and-sets = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "opens-and-sets";
    };
    vm-desktop-says-what-happens = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "says-what-happens";
    };
    vm-desktop-outlives-a-switch = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "outlives-a-switch";
    };
    vm-desktop-minds-the-margin = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "minds-the-margin";
    };
    vm-desktop-takes-calls = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "takes-calls";
    };
    vm-desktop-captures = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "captures";
    };
    vm-desktop-locks-and-asks = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "locks-and-asks";
    };
    vm-desktop-keeps-pace = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "keeps-pace";
    };
    # How the launcher keeps up with a held arrow while sessions and
    # people change, at Joel's scales, on a screen as large as the
    # laptop's.
    vm-desktop-launcher-keeps-up = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "launcher-keeps-up";
      screen = {
        width = 2880;
        height = 1800;
      };
    };
    # How the launcher is laid out at Joel's scales, which needs a screen
    # as large as the laptop's. Its rows, vi modes, plates and file picker
    # are areas of their own, so the five run side by side rather than one
    # taking half an hour.
    vm-desktop-looks = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "looks";
      screen = {
        width = 2880;
        height = 1800;
      };
    };
    vm-desktop-looks-rows = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "looks-rows";
      screen = {
        width = 2880;
        height = 1800;
      };
    };
    vm-desktop-looks-vi = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "looks-vi";
      screen = {
        width = 2880;
        height = 1800;
      };
    };
    vm-desktop-looks-plates = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "looks-plates";
      screen = {
        width = 2880;
        height = 1800;
      };
    };
    vm-desktop-looks-file-picker = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "looks-file-picker";
      screen = {
        width = 2880;
        height = 1800;
      };
    };
    # How Machine's things look at Joel's scales, from the machine daemon,
    # and Machines, from the realm's node, on the same screen: an area of
    # its own, so the looks areas run side by side.
    vm-desktop-looks-machine = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "looks-machine";
      screen = {
        width = 2880;
        height = 1800;
      };
    };
    # How Joel's programs' own pages look at his scales, each a subtest of
    # its own: an area of its own, so it runs beside the looks areas.
    vm-desktop-looks-programs = import ./tests/desktop.nix {
      inherit self pkgs;
      area = "looks-programs";
      screen = {
        width = 2880;
        height = 1800;
      };
    };
    # Whether the shell's surfaces come back after a screen goes shows only
    # with niri on the screen itself, not nested in Cage. Each area is a
    # machine of its own, so they run side by side (tests/screens.nix).
    vm-screens-come-and-go = import ./tests/screens.nix {
      inherit self pkgs;
      area = "come-and-go";
    };
    vm-screens-keep-places = import ./tests/screens.nix {
      inherit self pkgs;
      area = "keep-places";
    };
    vm-screens-keep-the-launcher = import ./tests/screens.nix {
      inherit self pkgs;
      area = "keep-the-launcher";
    };
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
      formatterExcludes
      bareNixos
      bareDarwin
      evaluates
      same
      holds
      failure
      account
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
      vmHostPkgs
      ;
  };

  # What each check proves (check-descriptions.nix), each a sentence however
  # its lines are wrapped.
  descriptions = lib.mapAttrs (
    _: text: lib.concatStringsSep " " (lib.splitString "\n" (lib.trim text))
  ) (import ./check-descriptions.nix);

  # Every jitsusama.* option a Linux machine can set, NixOS's and
  # home-manager's, read from the two Linux examples, which between them
  # import every module Linux has. A hidden option is an old name kept for
  # the machines still setting it, which nobody reads in the docs, so it
  # isn't held to the glossary.
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
          lib.filter (option: option.visible or true != false) (
            lib.collect lib.isOption (options.jitsusama or { })
          )
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
          # built on core holds its own (tests/repository/names.nix).
          no-name-says-a-replaced-word = self.lib.namesSayTheGlossary pkgs (
            { names = named ++ optionNames; } // import ./tests/repository/names-kept.nix
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

      # Only an aarch64 builder makes its image, so here it's evaluated.
      the-apple-silicon-stick-evaluates = pkgs.writeText "the-apple-silicon-stick-evaluates" (
        builtins.unsafeDiscardStringContext self.packages.aarch64-linux.stick.drvPath
      );
    }
    (onLinux vmTests)
  ]
  ++ map (file: import file given) [
    ./examples/checks.nix
    ./hardware/apple-macbook-pro-m4-pro-a3401/checks.nix
    ./hardware/dell-xps-14-da14260/checks.nix
    ./hardware/microsoft-surface-laptop-4-1958/checks.nix
    ./modules/agents/checks.nix
    ./modules/backlight/checks.nix
    ./modules/bat/checks.nix
    ./modules/bluetooth/checks.nix
    ./modules/bluetooth-disk-prompt/checks.nix
    ./modules/bluetooth-links/checks.nix
    ./modules/btop/checks.nix
    ./modules/btrfs/checks.nix
    ./modules/cadence/checks.nix
    ./modules/call-transcripts/checks.nix
    ./modules/calls/checks.nix
    ./modules/camera/checks.nix
    ./modules/cargo/checks.nix
    ./modules/chrome/checks.nix
    ./modules/claude/checks.nix
    ./modules/clippy/checks.nix
    ./modules/code-health/checks.nix
    ./modules/collector/checks.nix
    ./modules/console/checks.nix
    ./modules/creative-pebble-x-plus/checks.nix
    ./modules/daemon-client/checks.nix
    ./modules/daemon-serve/checks.nix
    ./modules/ddcutil/checks.nix
    ./modules/desk-facts/checks.nix
    ./modules/device-check/checks.nix
    ./modules/dictation/checks.nix
    ./modules/disk-guard/checks.nix
    ./modules/disk-loss-panic/checks.nix
    ./modules/disko/checks.nix
    ./modules/eero/checks.nix
    ./modules/ergodox/checks.nix
    ./modules/file-link-keys/checks.nix
    ./modules/firmware-notice/checks.nix
    ./modules/firmware-settings/checks.nix
    ./modules/fonts/checks.nix
    ./modules/gcloud-broker/checks.nix
    ./modules/gimp/checks.nix
    ./modules/git/checks.nix
    ./modules/git-engine/checks.nix
    ./modules/github-broker/checks.nix
    ./modules/google/checks.nix
    ./modules/gtk/checks.nix
    ./modules/guide/checks.nix
    ./modules/health/checks.nix
    ./modules/hibernate/checks.nix
    ./modules/inkscape/checks.nix
    ./modules/inlyne/checks.nix
    ./modules/ir-camera/checks.nix
    ./modules/kernel/checks.nix
    ./modules/key-counts/checks.nix
    ./modules/keys/checks.nix
    ./modules/kitty/checks.nix
    ./modules/kmscon/checks.nix
    ./modules/lag-watch/checks.nix
    ./modules/load-proof/checks.nix
    ./modules/lanzaboote/checks.nix
    ./modules/libvirt/checks.nix
    ./modules/lid/checks.nix
    ./modules/lid-hinge/checks.nix
    ./modules/link-keys/checks.nix
    ./modules/linux-builder/checks.nix
    ./modules/linux-id/checks.nix
    ./modules/litra/checks.nix
    ./modules/machine/checks.nix
    ./modules/meaning/checks.nix
    ./modules/memory-modules/checks.nix
    ./modules/mic-mute-light/checks.nix
    ./modules/mpv/checks.nix
    ./modules/mycelium/checks.nix
    ./modules/name-service/checks.nix
    ./modules/neovim/checks.nix
    ./modules/network-revive/checks.nix
    ./modules/nftables/checks.nix
    ./modules/night-shift/checks.nix
    ./modules/niri/checks.nix
    ./modules/nix-daemon/checks.nix
    ./modules/nix-offload/checks.nix
    ./modules/noise-cancelled/checks.nix
    ./modules/notices/checks.nix
    ./modules/onepassword/checks.nix
    ./modules/open-with/checks.nix
    ./modules/openssh/checks.nix
    ./modules/pain-points/checks.nix
    ./modules/panel/checks.nix
    ./modules/passwords/checks.nix
    ./modules/people/checks.nix
    ./modules/pi/checks.nix
    ./modules/pi/agent-proxy/checks.nix
    ./modules/pi-keys/checks.nix
    ./modules/pi-packages/checks.nix
    ./modules/pipewire/checks.nix
    ./modules/plymouth/checks.nix
    ./modules/pnpm/checks.nix
    ./modules/power-profiles-daemon/checks.nix
    ./modules/processor-telemetry/checks.nix
    ./modules/programs/checks.nix
    ./modules/prompt/checks.nix
    ./modules/qt/checks.nix
    ./modules/quests/checks.nix
    ./modules/quickshell/checks.nix
    ./modules/reader/checks.nix
    ./modules/ready-rows/checks.nix
    ./modules/sccache/checks.nix
    ./modules/screen-text/checks.nix
    ./modules/session-backup/checks.nix
    ./modules/shaper/checks.nix
    ./modules/signing/checks.nix
    ./modules/slack/checks.nix
    ./modules/slices/checks.nix
    ./modules/sound/checks.nix
    ./modules/spend-watch/checks.nix
    ./modules/spend-watch/feeds/claude/checks.nix
    ./modules/sudo/checks.nix
    ./modules/spotify/checks.nix
    ./modules/ssh-tpm-agent/checks.nix
    ./modules/sun/checks.nix
    ./modules/swayidle/checks.nix
    ./modules/swayimg/checks.nix
    ./modules/switch-system/checks.nix
    ./modules/systemd/checks.nix
    ./modules/systemd-networkd/checks.nix
    ./modules/tcp/checks.nix
    ./modules/theme/checks.nix
    ./modules/to-do/checks.nix
    ./modules/touchpad/checks.nix
    ./modules/treefmt/checks.nix
    ./modules/tuple/checks.nix
    ./modules/unlisted/checks.nix
    ./modules/update-to-main/checks.nix
    ./modules/v4l2loopback/checks.nix
    ./modules/warp/checks.nix
    ./modules/wezterm/checks.nix
    ./modules/wifid/checks.nix
    ./modules/windows/checks.nix
    ./modules/yubikey/checks.nix
    ./modules/zathura/checks.nix
    ./modules/zoom/checks.nix
    ./modules/zoxide/checks.nix
    ./modules/zsh/checks.nix
    ./tests/lib/checks.nix
    ./tests/repository/checks.nix
    ./tests/rules/checks.nix
    ./tests/vm/admin/checks.nix
    ./tests/vm/boot/checks.nix
    ./tests/vm/devices/checks.nix
    ./tests/vm/disks/checks.nix
    ./tests/vm/guide/checks.nix
    ./tests/vm/keys/checks.nix
    ./tests/vm/network/checks.nix
    ./tests/vm/sound/checks.nix
    ./tools/changed-checks/checks.nix
    ./tools/merge-net/checks.nix
    ./tools/run-subtests/checks.nix
    ./tools/shell-preview/checks.nix
  ]
)
