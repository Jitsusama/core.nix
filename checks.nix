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

  # Most of the icon font's marks lie past U+FFFF, which a single JSON escape
  # can't reach: written as one, U+F075D read as U+F075 and a stray "d".
  icon-marks-past-the-basic-plane =
    let
      nerd = import ./modules/theme/nerd.nix { inherit lib; };
    in
    holds "icon-marks-past-the-basic-plane" "An icon mark past U+FFFF came out as two characters." (
      nerd "f075d" != nerd "f075" + "d" && builtins.stringLength (nerd "f075d") == 4
    );

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

  # A machine shapes what the overview shows in a line a change: a piece
  # moved, cut, shown only while true, a threshold changed and the rest kept,
  # and a piece of its own linked beside the built-in ones.
  the-overview-takes-its-pieces =
    let
      own = pkgs.writeText "wide.qml" "Piece { value: settings.text }";
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        {
          home-manager.users.someone.jitsusama.quickshell.overview.pieces = {
            date.tier = null;
            network.order = 25;
            battery.low = 15;
            volume.tier = "when";
            wide = {
              order = 60;
              source = own;
              label = "wide";
              text = "a piece of the machine's own";
              collapse = 5;
              mark = "on";
            };
          };
        }
      ];
      home = account machine;
      rendered = lib.findFirst (lib.hasInfix "property var pieces: ") "" (
        lib.splitString "\n" home.xdg.configFile."quickshell/Overview.qml".text
      );
      drawn = builtins.fromJSON (lib.last (lib.splitString "property var pieces: " rendered));
      piece = lib.listToAttrs (map (piece: lib.nameValuePair piece.name piece) drawn);
    in
    holds "the-overview-takes-its-pieces"
      "The overview didn't draw a machine's own pieces as it set them: their order, their cuts, their settings or its own file."
      (
        map (piece: piece.name) drawn == [
          "time"
          "network"
          "battery"
          "volume"
          "notices"
          "quiet"
          "awake"
          "wide"
          "microphone"
          "capslock"
          "sharing"
          "recording"
        ]
        && piece.battery.low == 15
        && piece.battery.critical == 10
        && piece.volume.tier == "when"
        && piece.wide.mark == "on"
        && piece.wide.file == "wide.qml"
        && "${home.xdg.configFile."quickshell/pieces/wide.qml".source}" == "${own}"
      );

  # The shell hands every program it opens to niri, which gives it a scope of
  # its own. One the shell started itself would leave its helpers in the
  # shell's service, and restarting the shell would kill them.
  quickshell-opens-programs-through-niri =
    let
      dir = ./modules/quickshell;
      qml = lib.filter (lib.hasSuffix ".qml") (lib.attrNames (builtins.readDir dir));
      starts = file: lib.hasInfix "execDetached" file || lib.hasInfix ".execute()" file;
      culprits = lib.filter (name: starts (builtins.readFile (dir + "/${name}"))) qml;
    in
    holds "quickshell-opens-programs-through-niri"
      "These start programs themselves rather than through Niri.spawn: ${lib.concatStringsSep ", " culprits}"
      (qml != [ ] && culprits == [ ]);

  # The launcher finds what's meant: for each query in tests/fuzzy.qml the
  # right thing comes first, few rows are left, or nothing answers, and a
  # query keeps within its budget, all in Qt's own JavaScript engine, the
  # one the shell runs the matcher in.
  the-launcher-finds-what-is-meant =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-finds-what-is-meant" { } ''
      cp ${./tests/fuzzy.qml} fuzzy.qml
      cp ${./modules/quickshell/Fuzzy.js} Fuzzy.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml fuzzy.qml
      touch $out
    '';

  # The launcher finds emoji by what they're called and what they mean, best
  # first, as tests/emoji.qml lists them, over the list it's built with, and
  # in about three times the least a search could take.
  the-launcher-finds-emoji =
    let
      qt = pkgs.qt6.qtdeclarative;
      emoji = import ./modules/quickshell/launcher/emoji.nix { inherit lib pkgs; } "none";
    in
    pkgs.runCommandLocal "the-launcher-finds-emoji" { } ''
      mkdir launcher
      cp ${./tests/emoji.qml} emoji.qml
      cp ${./modules/quickshell/Fuzzy.js} Fuzzy.js
      cp ${./modules/quickshell/launcher/Emoji.js} launcher/Emoji.js
      cp ${emoji} emoji.jsonl
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export QML_XHR_ALLOW_FILE_READ=1
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml emoji.qml
      touch $out
    '';

  # The launcher answers what's asked: sums and number bases, IPv4 and IPv6
  # addresses and networks with the kind of range each is in, hardware
  # addresses, measures and the time in any zone tzdata names, as
  # tests/answers.qml lists them, and leaves alone what isn't one, in Qt's
  # own JavaScript engine.
  the-launcher-answers-what-is-asked =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-answers-what-is-asked" { } ''
      cp ${./tests/answers.qml} answers.qml
      cp ${./modules/quickshell/launcher/Numbers.js} Numbers.js
      cp ${./modules/quickshell/launcher/Addresses.js} Addresses.js
      cp ${./modules/quickshell/launcher/Units.js} Units.js
      cp ${./modules/quickshell/launcher/Zones.js} Zones.js
      cp ${./modules/quickshell/launcher/Colours.js} Colours.js
      cp ${./tests/colour-cases.js} colour-cases.js
      cp ${import ./modules/quickshell/launcher/zones.nix { inherit lib pkgs; }} zones.jsonl
      export QML_XHR_ALLOW_FILE_READ=1
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml answers.qml
      touch $out
    '';

  # Do not disturb holds notices while the screen is shared and lets them go
  # after, as tests/quiet.qml drives it through Quickshell itself, with
  # niri's casts, the notices and the shell's options stood in for.
  do-not-disturb-holds-while-sharing =
    let
      singleton =
        name: body:
        pkgs.writeText name ''
          pragma Singleton
          import Quickshell
          import QtQuick
          Singleton { ${body} }
        '';
      options = singleton "Options.qml" "readonly property var quiet: ({ whileSharing: true })";
    in
    pkgs.runCommandLocal "do-not-disturb-holds-while-sharing" { } ''
      mkdir runtime && chmod 700 runtime
      cp ${./modules/quickshell/Quiet.qml} Quiet.qml
      cp ${./tests/quiet.qml} shell.qml
      cp ${singleton "Niri.qml" "property var casts: ({})"} Niri.qml
      cp ${singleton "Notices.qml" "property var history: []"} Notices.qml
      cp ${options} Options.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      timeout 60 ${lib.getExe' pkgs.quickshell "qs"} -p shell.qml
      touch $out
    '';

  # BlueZ says a new device is connected once its link is up, before it's
  # paired, so the launcher's Bluetooth would stop following a pairing early
  # and leave the device untrusted. The VM has no radio, so this drives it
  # through Quickshell itself with stand-in devices (tests/bluetooth-pairing.qml).
  bluetooth-pairs-before-it-connects =
    pkgs.runCommandLocal "bluetooth-pairs-before-it-connects" { }
      ''
        mkdir runtime launcher && chmod 700 runtime
        cp ${./modules/quickshell/Fuzzy.js} Fuzzy.js
        cp ${./modules/quickshell/launcher/Provider.qml} launcher/Provider.qml
        cp ${./modules/quickshell/launcher/Accessories.qml} launcher/Accessories.qml
        cp ${./tests/bluetooth-pairing.qml} shell.qml
        cp ${pkgs.writeText "Niri.qml" ''
          pragma Singleton
          import Quickshell
          import QtQuick
          Singleton { function spawn(command) {} }
        ''} Niri.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        timeout 60 ${lib.getExe' pkgs.quickshell "qs"} -p shell.qml
        touch $out
      '';

  # Shape is a language (docs/design.md, Shape and Kinds): a border's width
  # says "this is a thing" and a radius says whole or part, so no surface
  # picks either for itself. Every corner is the theme's radius, or a part's
  # radius the theme derives from it, and every border is the theme's.
  quickshell-takes-its-shape-from-the-theme =
    let
      dir = ./modules/quickshell;
      files =
        lib.filter (lib.hasSuffix ".qml") (lib.attrNames (builtins.readDir dir))
        ++ map (name: "pieces/${name}") (
          lib.filter (lib.hasSuffix ".qml") (lib.attrNames (builtins.readDir (dir + "/pieces")))
        );
      allowed = {
        radius = [
          "Theme.radius"
          "Theme.inner"
        ];
        width = [ "Theme.border" ];
      };
      shape =
        line:
        let
          space = "[[:space:]]*";
          property = "(border\\.width|[a-zA-Z]*[rR]adius)";
          value = "([^;]*[^;[:space:]])";
          found = builtins.match (space + property + space + ":" + space + value + space + ";?" + space) line;
        in
        if found == null then
          null
        else
          {
            kind = if lib.head found == "border.width" then "width" else "radius";
            value = lib.elemAt found 1;
          };
      culprits = lib.concatMap (
        name:
        lib.concatMap (
          line:
          let
            found = shape line;
          in
          lib.optional (
            found != null && !(lib.elem found.value allowed.${found.kind})
          ) "${name}: ${lib.trim line}"
        ) (lib.splitString "\n" (builtins.readFile (dir + "/${name}")))
      ) files;
    in
    holds "quickshell-takes-its-shape-from-the-theme" ''
      These pick a radius or a border width of their own rather than the theme's:
      ${lib.concatStringsSep "\n" culprits}'' (files != [ ] && culprits == [ ]);

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
      # The launcher's colour answers, and the colours they're tested on, name
      # colours because colours are what's asked; neither is a look.
      if rg --line-number --no-heading --hidden --glob '!modules/theme/**' \
        --glob '!modules/quickshell/launcher/Colours.js' --glob '!tests/colour-cases.js' \
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

  # Nix's builds yield to the user's programs only if their slice sits beside
  # user.slice, at a lower weight. A dash in its name nests it in a parent at
  # the default weight, and nothing says so.
  nix-builds-yield-to-the-user =
    let
      inherit (workstation.config) systemd;
      slice = systemd.services.nix-daemon.serviceConfig.Slice;
      name = lib.removeSuffix ".slice" slice;
    in
    holds "nix-builds-yield-to-the-user"
      "The Nix daemon's slice, ${slice}, isn't beside user.slice at a lower weight."
      (
        !(lib.hasInfix "-" name)
        &&
          (systemd.slices.${name}.sliceConfig.CPUWeight or 100)
          < (systemd.slices.user.sliceConfig.CPUWeight or 100)
      );

  # greetd waits for the GPU the hardware names rather than for the boot to go
  # idle, and the card gets the device unit it waits on. Without the udev tag
  # the unit never appears, and the login waits out the device timeout.
  greetd-waits-for-the-gpu =
    let
      xps = bareNixos [
        self.nixosModules.graphical
        self.nixosModules.dell-xps-14-da14260
      ];
      inherit (xps.config) jitsusama services systemd;
      greetd = systemd.services.greetd;
    in
    holds "greetd-waits-for-the-gpu"
      "greetd doesn't wait for ${jitsusama.gpu.unit}, nothing tags the card for systemd, or nothing caps the wait."
      (
        greetd.serviceConfig.Type == "simple"
        && lib.hasInfix "JobRunningTimeoutSec=15s" systemd.units.${jitsusama.gpu.unit}.text
        && lib.elem jitsusama.gpu.unit greetd.wants
        && lib.elem jitsusama.gpu.unit greetd.after
        && lib.hasInfix ''ENV{ID_PATH}=="${jitsusama.gpu.path}", TAG+="systemd"'' services.udev.extraRules
        && graphical.config.systemd.services.greetd.serviceConfig.Type == "idle"
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

    # GIMP reads the files an account gets as it would at start: its system
    # preferences, where only GIMP's own copy of gimprc says a key exists; the
    # theme's palette, by name, with every pigment; and the stylesheet, which
    # GTK, GIMP's own copy, refuses if it can't parse it. A colour GIMP's
    # theme stops naming would be redefined to no effect, so the names have to
    # be the theme's exactly.
    gimp-accepts-its-configuration =
      let
        home = account graphical;
        inherit (home.jitsusama.theme) title appearance palette;
        gimp = lib.findFirst (package: lib.getName package == "gimp") null home.home.packages;
        folder = "${home.xdg.configHome}/GIMP/3.2";
        files = home.home.file;
        reads = pkgs.writeText "reads.py" ''
          import gi
          gi.require_version("Gtk", "3.0")
          from gi.repository import Gimp, Gtk
          assert Gimp.gimprc_query("theme-color-scheme") == "${appearance}", "no colour scheme"
          assert Gimp.gimprc_query("show-help-button") == "no", "no system gimprc"
          palette = Gimp.Palette.get_by_name("${title}")
          assert palette is not None, "no ${title} palette"
          assert palette.get_color_count() == ${toString (lib.length (lib.attrNames palette))}, "pigments missing"
          Gtk.CssProvider().load_from_path("GIMP/3.2/gimp.css")
          open("read", "w").close()
        '';
      in
      pkgs.runCommandLocal "gimp-accepts-its-configuration" { nativeBuildInputs = [ gimp ]; } ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD
        mkdir -p GIMP/3.2/palettes
        cp ${files."${folder}/gimp.css".source} GIMP/3.2/gimp.css
        cp ${files."${folder}/palettes/${title}.gpl".source} "GIMP/3.2/palettes/${title}.gpl"
        gimp-console -i --no-fonts --batch-interpreter=python-fu-eval -b - --quit < ${reads} \
          > said 2>&1 || true
        if [ ! -e read ] || grep -i 'gimprc' said; then cat said; exit 1; fi
        grep -o '@define-color [a-z-]*' ${gimp}/share/gimp/3.0/themes/Default/gimp-${appearance}.css \
          | sort > theirs
        grep -o '@define-color [a-z-]*' GIMP/3.2/gimp.css | sort > ours
        diff theirs ours
        touch $out
      '';

    # Inkscape starts a new drawing from the account's profile, on the theme's
    # desk: a template it can't find leaves its own grey one without a word.
    # Inkscape draws nothing without a screen, so GTK reads the stylesheet
    # instead, refusing what it can't parse, and every name it recolours has
    # to be one Inkscape's own stylesheets still use.
    inkscape-accepts-its-configuration =
      let
        home = account graphical;
        inherit (home.jitsusama.theme) roles;
        inkscape = lib.findFirst (
          package: lib.getName package == "inkscape-with-extensions"
        ) null home.home.packages;
        profile = "${home.xdg.configHome}/inkscape";
        files = home.home.file;
        python = pkgs.python3.withPackages (packages: [ packages.pygobject3 ]);
        typelibs = lib.makeSearchPath "lib/girepository-1.0" [
          pkgs.gtk3
          pkgs.gdk-pixbuf
          pkgs.pango.out
          pkgs.harfbuzz
          pkgs.at-spi2-core
          pkgs.gobject-introspection
        ];
        parses = pkgs.writeText "parses.py" ''
          import gi, sys
          gi.require_version("Gtk", "3.0")
          from gi.repository import Gtk
          Gtk.CssProvider().load_from_path(sys.argv[1])
        '';
      in
      pkgs.runCommandLocal "inkscape-accepts-its-configuration"
        {
          nativeBuildInputs = [
            inkscape
            python
          ];
        }
        ''
          export HOME=$PWD INKSCAPE_PROFILE_DIR=$PWD/profile
          mkdir -p profile/ui profile/templates
          cp ${files."${profile}/ui/user.css".source} profile/ui/user.css
          cp ${files."${profile}/templates/default.svg".source} profile/templates/default.svg
          inkscape --actions='file-new;export-filename:new.svg;export-do' > said 2>&1 || true
          if ! grep -q 'inkscape:deskcolor="${roles.sunken}"' new.svg; then cat said; exit 1; fi
          GI_TYPELIB_PATH=${typelibs} python3 ${parses} profile/ui/user.css
          sed -n -E 's/^ *([^ /*].*[^ ]) *[{,]$/\1/p' profile/ui/user.css > selectors
          while read -r selector; do
            if ! grep -q -F -- "$selector" ${inkscape}/share/inkscape/ui/*.css; then
              echo "Inkscape's stylesheets no longer use $selector"
              exit 1
            fi
          done < selectors
          touch $out
        '';

    # Spotify's client is built in the theme: Spicetify patches the client
    # it's given, and one it can't patch, such as a newer Spotify than it
    # knows, would draw in Spotify's own colours. So the check looks for the
    # theme's background and faces in the client the account gets, and for
    # the page loading them.
    spotify-draws-in-the-theme =
      let
        home = account graphical;
        inherit (home.jitsusama.theme) roles font;
        client = "${home.programs.spicetify.spicedSpotify}/share/spotify/Apps/xpui";
      in
      pkgs.runCommandLocal "spotify-draws-in-the-theme" { } ''
        grep -q -F -- '--spice-main: ${lib.toLower roles.background};' ${client}/colors.css
        grep -q -F -- '--encore-body-font-stack: "${font.sans.family}"' ${client}/user.css
        grep -q "href='colors.css'" ${client}/index.html
        grep -q "href='user.css'" ${client}/index.html
        touch $out
      '';

    # spotify-player reads its settings before anything else and stops at a
    # value it can't use; it ignores a setting it doesn't know.
    spotify-player-accepts-its-configuration =
      let
        home = account graphical;
        files = home.xdg.configFile;
      in
      pkgs.runCommandLocal "spotify-player-accepts-its-configuration"
        { nativeBuildInputs = [ home.programs.spotify-player.package ]; }
        ''
          export HOME=$PWD
          mkdir config cache
          cp ${files."spotify-player/app.toml".source} config/app.toml
          spotify_player -c config -C cache features
          touch $out
        '';

    # Tuple shares a screen through the portal: the switch writes the setting
    # where Tuple keeps it, and Tuple, which ignores a folder it doesn't know
    # and a value not written as it writes one, has to read it back. So the
    # check runs the switch's own steps, with the data folder moved here.
    tuple-shares-through-the-portal =
      let
        home = account graphical;
        tuple = lib.findFirst (package: lib.getName package == "tuple") null home.home.packages;
        data = home.xdg.dataHome;
      in
      pkgs.runCommandLocal "tuple-shares-through-the-portal" { nativeBuildInputs = [ tuple ]; } ''
        export HOME=$PWD XDG_DATA_HOME=$PWD/data XDG_RUNTIME_DIR=$PWD/run
        mkdir -m 700 run
        sed -n '\|^run .*${lib.escapeRegex data}/tuple/|{s|^run ||;s|${data}|'"$PWD"'/data|g;p}' \
          ${home.home.activationPackage}/activate > steps
        sh -e steps
        tuple settings > said 2>&1 || true
        tuple off > /dev/null 2>&1 || true
        if ! grep -q '^capture | portal |' said; then cat said; exit 1; fi
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
