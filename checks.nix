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
  # judged. A test exits 2 when it was only too slow; any other failure
  # stops at once.
  timedQml =
    file:
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    ''
      for run in 1 2 3 4 5; do
        status=0
        QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml ${file} || status=$?
        [ "$status" = 0 ] && break
        [ "$status" = 2 ] || exit "$status"
        echo "too slow in run $run of 5"
        [ "$run" != 5 ] || exit 1
      done
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
  # minutes; the kernel itself takes far longer than a check should.
  kernelSettingsHold =
    name: kernel:
    let
      # The kernel's own, as package.nix gave them, so a machine's theme
      # colours are held as well as the settings every kernel shares.
      settings = lib.mapAttrs (_option: setting: setting.content or setting) kernel.structuredExtraConfig;
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
    # A passkey is the TPM, uhid, udev's seat rules and the shell's askpass
    # together, and only a running machine has them all.
    vm-passkeys-come-from-the-tpm = import ./tests/linux-id.nix { inherit self pkgs; };
    vm-keyring-opens-without-asking = import ./tests/gnome-keyring.nix { inherit self pkgs; };
    vm-speakers-are-tuned = import ./tests/speakers.nix { inherit pkgs; };
    vm-desktop-works = import ./tests/desktop.nix { inherit self pkgs; };
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
    vm-boot-asks-in-the-theme = import ./tests/plymouth.nix { inherit self pkgs; };
    # Whether libvirt's daemon stops in time shows only with the daemon
    # running and its threads short of CPU, as at a busy shutdown.
    vm-libvirt-stops-quickly = import ./tests/libvirt.nix { inherit self pkgs; };
    # Whether the IR camera opens only while it's read, and hands each reader
    # its frames and none of the last one's, shows only with cameras there.
    vm-ir-camera-opens-for-readers = import ./tests/ir-camera.nix { inherit self pkgs; };
    # Whether the swap comes down while udev probes its device shows only with
    # the device there and held open.
    vm-swap-comes-down-while-probed = import ./tests/zram.nix { inherit self pkgs; };
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

  # Every program's rules sit in nftables, each in a table of its own, and a
  # reload deletes only the firewall's: flushing the ruleset would take the
  # others' tables, and their traffic, with it. A raw ruleset is what turns
  # nixpkgs' flush on, so the machine here adds one.
  one-ruleset-for-every-program =
    let
      machine =
        (workstation.extendModules {
          modules = [ { networking.nftables.ruleset = "table inet probe { }"; } ];
        }).config;
      containers = machine.virtualisation.containers.containersConf.settings;
    in
    holds "one-ruleset-for-every-program"
      "A program's rules leave nftables, or a reload flushes the others' tables."
      (
        machine.networking.firewall.enable
        && machine.networking.nftables.enable
        && !machine.networking.nftables.flushRuleset
        && machine.virtualisation.libvirtd.firewallBackend == "nftables"
        && containers.network.firewall_driver == "nftables"
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

  # Joel plugs the ErgoDox into every machine he writes code on, so every
  # account on one can flash it, and on Linux its devices are open to him.
  ergodox-on-every-workstation =
    let
      hasErgodox =
        machine: lib.any (package: lib.getName package == "ergodox") (account machine).home.packages;
    in
    holds "ergodox-on-every-workstation" "A workstation can't flash the ErgoDox." (
      hasErgodox workstation
      && hasErgodox macWorkstation
      && workstation.config.hardware.keyboard.zsa.enable
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

  # Qt programs take GTK's look from niri's environment, and Quickshell, which
  # draws from the theme itself, is started without it.
  qt-programs-wear-the-theme =
    let
      home = account graphical;
      service = home.systemd.user.services.quickshell.Service;
    in
    holds "qt-programs-wear-the-theme"
      "Qt programs don't get GTK's platform theme, or Quickshell gets it too."
      (
        home.systemd.user.sessionVariables.QT_QPA_PLATFORMTHEME or null == "gtk3"
        && lib.elem "QT_QPA_PLATFORMTHEME" (service.UnsetEnvironment or [ ])
      );

  # Claude Code's theme is drawn from the machine's, every colour a hex one
  # its format reads, and a diff's tint sits between the background and the
  # colour it's a tint of.
  claude-wears-the-theme =
    let
      home = account workstation;
      theme = builtins.fromJSON home.home.file.".claude/themes/jitsusama.json".text;
      colours = lib.attrValues theme.overrides;
      mix = import ./modules/theme/mix.nix { inherit lib; };
    in
    holds "claude-wears-the-theme" "Claude Code's theme isn't the machine's, or mixes wrong." (
      theme.overrides.text == home.jitsusama.theme.roles.text
      && lib.all (colour: builtins.match "#[0-9a-fA-F]{6}" colour != null) colours
      && mix 0.5 "#000000" "#ffffff" == "#808080"
      && mix 0.25 "#ff0000" "#000000" == "#400000"
    );

  # pi's theme is the machine's: its text, its code and its thinking levels
  # are the theme's colours, and every colour it names is one pi reads.
  pi-wears-the-theme =
    let
      home = account workstation;
      theme = builtins.fromJSON home.home.file.".pi/agent/themes/jitsusama.json".text;
      colours = lib.attrValues theme.colors ++ lib.attrValues theme.export;
    in
    holds "pi-wears-the-theme" "pi's theme isn't the machine's, or names a colour pi can't read." (
      theme.colors.text == home.jitsusama.theme.roles.text
      && theme.colors.syntaxString == home.jitsusama.theme.syntax.string.colour
      && theme.colors.thinkingMax == home.jitsusama.theme.roles.alert
      && lib.all (colour: builtins.match "#[0-9a-fA-F]{6}" colour != null) colours
    );

  # pi's status line tells a folder and git as the prompt does: by the
  # prompt's root markers, the prompt's git-engine, and the theme's marks,
  # each by the name the harness reads it under.
  pi-tells-where-as-the-prompt-does =
    let
      home = account workstation;
      where = builtins.fromJSON (
        builtins.unsafeDiscardStringContext
          home.xdg.configFile."pi/agentic-harness.pi/status-line/where.json".text
      );
      inherit (home.jitsusama.theme) glyphs;
      engine = lib.getExe (pkgs.callPackage ./modules/git-engine/package.nix { });
    in
    holds "pi-tells-where-as-the-prompt-does"
      "pi's status line tells the folder or git by something other than the prompt's roots, engine and marks."
      (
        where.roots == home.jitsusama.prompt.roots
        && where.engine == builtins.unsafeDiscardStringContext engine
        &&
          where.marks == {
            branch = glyphs.branch.glyph;
            unsaved = glyphs.on.glyph;
            unpushed = glyphs.unpushed.glyph;
            more = glyphs.more.glyph;
            complete = glyphs.complete.glyph;
            active = glyphs.active.glyph;
            pending = glyphs.pending.glyph;
            thinking = glyphs.thinking.glyph;
          }
      );

  # The prompt starts nothing once the shell is up: across a run of prompts
  # in and out of a repository, with git-engine answering, the shell itself
  # forks and executes nothing, and its reads of HEAD show the prompts ran.
  # Starting up, it runs only git-engine and the mkfifo it talks through.
  the-prompt-starts-nothing =
    pkgs.runCommandLocal "the-prompt-starts-nothing"
      {
        nativeBuildInputs = [
          pkgs.git
          pkgs.strace
          pkgs.util-linux
          pkgs.zsh
        ];
      }
      ''
        export HOME=$TMPDIR/home TERM=xterm-256color
        mkdir -p $HOME/zsh repo/a/b
        echo "source ${(account workstation).jitsusama.prompt.file}" > $HOME/zsh/.zshrc
        git -C repo init -q -b main
        git init -q --ref-format=reftable -b main table
        touch repo/a/b/file
        # Typed ahead, each line a prompt; the two reads mark where to count.
        printf '%s\r' 'cd repo' ': < /dev/null' 'cd a/b' true false 'cd ../../../table' true \
          'cd ..' ': < /dev/zero' exit > keys
        ZDOTDIR=$HOME/zsh script -q -c \
          "stty cols 90 rows 30; strace -f -qq -o trace -e trace=process,openat zsh -i" \
          /dev/null < keys > /dev/null
        shell=$(head -1 trace | cut -d' ' -f1)
        awk -v shell=$shell '
          $1 == shell && /\/dev\/null/ { counting = 1 }
          $1 == shell && /\/dev\/zero/ { counting = 0 }
          counting && $1 == shell
        ' trace > prompts
        # Each program the shell ran, by name; a PATH lookup's misses fail.
        ran=$(grep 'execve("' trace | grep -v '= -1 ' | sed 's|.*execve("\([^"]*\)".*|\1|; s|.*/||' |
          sort -u | tr '\n' ' ')
        [ "$ran" = "git-engine mkfifo zsh " ] || { echo "The shell ran: $ran" >&2; exit 1; }
        grep -q HEAD prompts || { echo "No prompt read the repository." >&2; exit 1; }
        grep -q tables.list prompts || { echo "No prompt read the reftable." >&2; exit 1; }
        if grep -E 'clone|fork|execve' prompts; then
          echo "A prompt started a process." >&2
          exit 1
        fi
        touch $out
      '';

  # The account's whole shell, not only its prompt: once its completion dump
  # is made, starting it runs git-engine, its fifo and direnv's first look,
  # and each prompt runs direnv alone, asking it whether the folder's .envrc
  # changed. Anything else a shell started would show here. Its history is
  # kept in the sandbox's home, since the account's own isn't there to hold
  # it.
  the-shell-starts-nothing =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        { home-manager.users.someone.programs.zsh.history.path = "$HOME/.zsh_history"; }
      ];
    in
    pkgs.runCommandLocal "the-shell-starts-nothing"
      {
        nativeBuildInputs = [
          pkgs.strace
          pkgs.util-linux
          pkgs.zsh
        ];
      }
      ''
        export HOME=$TMPDIR/home TERM=xterm-kitty
        mkdir -p $HOME
        ln -s ${(account machine).home-files}/{.zshrc,.zshenv} $HOME/
        # Typed ahead, each line a prompt; the two reads mark where to count.
        touch begin end
        printf '%s\r' ": < $PWD/begin" true true true ": < $PWD/end" exit > keys
        start() {
          script -q -c "stty cols 90 rows 30; $1 zsh -i" /dev/null < keys > /dev/null
        }
        # The first start makes the completion dump the rest are read from.
        start ""
        start "strace -f -qq -o trace -e trace=process,openat"
        names() {
          grep 'execve("' | grep -v '= -1 ' | sed 's|.*execve("\([^"]*\)".*|\1|; s|.*/||' |
            sort -u | tr '\n' ' '
        }
        shell=$(head -1 trace | cut -d' ' -f1)
        mark() { grep -n "^$shell .*$1" trace | head -1 | cut -d: -f1; }
        started=$(head -n $(mark $PWD/begin) trace | names)
        prompts=$(sed -n "$(mark $PWD/begin),$(mark $PWD/end)p" trace | names)
        [ "$started" = "direnv git-engine mkfifo zsh " ] ||
          { echo "Starting the shell ran: $started" >&2; exit 1; }
        [ "$prompts" = "direnv " ] || { echo "Its prompts ran: $prompts" >&2; exit 1; }
        touch $out
      '';

  # git-engine answers as gitstatusd does across the states a prompt meets,
  # counting everything and stopping at one, some of them watched by git's
  # file system monitor, and as git does where gitstatusd can't read the
  # repository (modules/git-engine/tests/agrees.zsh). The plugin wants job
  # control, so it runs on a terminal of script's.
  git-engine-agrees-with-gitstatusd =
    let
      engine = pkgs.callPackage ./modules/git-engine/package.nix { };
      plugin = "${pkgs.gitstatus}/share/gitstatus";
    in
    pkgs.runCommandLocal "git-engine-agrees-with-gitstatusd"
      {
        nativeBuildInputs = [
          pkgs.git
          pkgs.util-linux
          pkgs.zsh
        ];
      }
      ''
        export HOME=$TMPDIR/home
        script -q -e -c "zsh -f -i ${./modules/git-engine/tests/agrees.zsh} ${plugin} \
          ${lib.getExe' pkgs.gitstatus "gitstatusd"} ${lib.getExe engine} $TMPDIR/work" \
          agrees.log < /dev/null > /dev/null || { tr -d '\r' < agrees.log >&2; exit 1; }
        tr -d '\r' < agrees.log | grep checked
        touch $out
      '';

  # The prompt gives way as tests/prompt.zsh says, without gitstatusd, which
  # needs job control a shell run this way hasn't got, in a repository that
  # keeps its refs in files and in one that keeps them in reftable, as World
  # does.
  the-prompt-gives-way =
    pkgs.runCommandLocal "the-prompt-gives-way"
      {
        nativeBuildInputs = [
          pkgs.git
          pkgs.zsh
        ];
      }
      ''
        export HOME=$TMPDIR/home
        mkdir -p $HOME repo/modules/btop
        grep -v '^typeset -g _prompt_gitstatus=' ${(account workstation).jitsusama.prompt.file} > prompt.zsh
        for refs in files reftable; do
          rm -rf repo
          mkdir -p repo/modules/btop
          git -C repo init -q --ref-format=$refs -b jitsusama/prompt-collapse
          mkdir repo/.git/rebase-merge
          echo 2 > repo/.git/rebase-merge/msgnum
          echo 5 > repo/.git/rebase-merge/end
          zsh -f -i -c \
            "source ${./tests/prompt.zsh} $PWD/prompt.zsh $PWD/repo/modules/btop jitsusama/prompt-collapse"
        done
        touch $out
      '';

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
          "spend"
          "notices"
          "quiet"
          "disk"
          "awake"
          "wide"
          "microphone"
          "capslock"
          "layer"
          "call"
          "sharing"
          "recording"
          "playing"
        ]
        && piece.battery.low == 15
        && piece.battery.critical == 10
        && piece.volume.tier == "when"
        && piece.wide.mark == "on"
        && piece.wide.file == "wide.qml"
        && "${home.xdg.configFile."quickshell/pieces/wide.qml".source}" == "${own}"
      );

  # qmllint reads the shell as the account gets it, as the module qs, against
  # Quickshell's own type files, and any warning fails. Two faults in those
  # files are corrected in a copy first: the blur module doesn't say it
  # depends on Quickshell, whose Region it names, and PanelWindow is marked
  # uncreatable because Quickshell types it as the interface it makes a real
  # window behind. Two kinds of report are left out, since each one here is
  # Quickshell's files, not the shell: a property typed by a name its files
  # don't export (AuthFlow, BluetoothAdapter and the like), and a handler
  # whose parameter is a Qt enum QML can't see (QProcess::ExitStatus).
  quickshell-passes-qmllint =
    let
      files =
        lib.filterAttrs (name: _: lib.hasPrefix "quickshell/" name)
          (account graphical).xdg.configFile;
      qt = pkgs.kdePackages.qtdeclarative;
      types = pkgs.runCommandLocal "quickshell-types" { } ''
        mkdir -p $out/lib/qt-6/qml
        cp -r ${pkgs.quickshell}/lib/qt-6/qml/Quickshell $out/lib/qt-6/qml/
        chmod -R u+w $out
        echo 'depends Quickshell' >> $out/lib/qt-6/qml/Quickshell/Wayland/_BackgroundEffect/qmldir
        substituteInPlace $out/lib/qt-6/qml/Quickshell/_Window/quickshell-window.qmltypes \
          --replace-fail 'exports: ["Quickshell._Window/PanelWindow 0.0"]
                isCreatable: false' 'exports: ["Quickshell._Window/PanelWindow 0.0"]'
      '';
    in
    # In UTF-8, which Qt otherwise switches to and says so.
    pkgs.runCommandLocal "quickshell-passes-qmllint" { LC_ALL = "C.UTF-8"; } ''
      ${lib.concatLines (
        lib.mapAttrsToList (name: file: ''
          mkdir -p "$(dirname ${lib.escapeShellArg name})"
          cp ${file.source} ${lib.escapeShellArg name}
        '') files
      )}
      mv quickshell qs
      # Quickshell finds a folder's files as a module by its path; qmllint
      # needs a qmldir saying so.
      find qs -type d | while read -r dir; do
        {
          echo "module ''${dir//\//.}"
          for file in "$dir"/[A-Z]*.qml; do
            [ -e "$file" ] || continue
            name=$(basename "$file" .qml)
            if grep -q '^pragma Singleton' "$file"; then
              echo "singleton $name 1.0 $name.qml"
            else
              echo "$name 1.0 $name.qml"
            fi
          done
        } > "$dir/qmldir"
      done
      ${qt}/bin/qmllint --ignore-settings --max-warnings 0 \
        -I ${types}/lib/qt-6/qml -I ${qt}/lib/qt-6/qml -I . \
        --unresolved-type disable --signal-handler-parameters disable \
        $(find qs -name '*.qml' | sort)
      touch $out
    '';

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
  # query keeps within its budget in at least one of five processes, all in
  # Qt's own JavaScript engine, the one the shell runs the matcher in.
  the-launcher-finds-what-is-meant = pkgs.runCommandLocal "the-launcher-finds-what-is-meant" { } ''
    cp ${./tests/fuzzy.qml} fuzzy.qml
    cp ${./modules/quickshell/Fuzzy.js} Fuzzy.js
    export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
    ${timedQml "fuzzy.qml"}
    touch $out
  '';

  # The launcher finds emoji by what they're called and what they mean, best
  # first, as tests/emoji.qml lists them, over the list it's built with, and
  # in about three times the least a search could take.
  the-launcher-finds-emoji =
    let
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
      ${timedQml "emoji.qml"}
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
      cp ${./modules/quickshell/launcher/Asking.js} Asking.js
      cp ${./tests/colour-cases.js} colour-cases.js
      cp ${import ./modules/quickshell/launcher/zones.nix { inherit lib pkgs; }} zones.jsonl
      export QML_XHR_ALLOW_FILE_READ=1
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml answers.qml
      touch $out
    '';

  # The region picker's keys start, move and size its box, and letters name
  # the grid's cells, as tests/picking.qml says, in Qt's own engine.
  the-picker-picks-what-the-keys-say =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-picker-picks-what-the-keys-say" { } ''
      cp ${./tests/picking.qml} picking.qml
      cp ${./modules/quickshell/Picking.js} Picking.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml picking.qml
      touch $out
    '';

  # The askpass prompt says who's asking for what, for each program's words,
  # as tests/askpass.qml says, in Qt's own engine.
  the-askpass-says-who-asks-for-what =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-askpass-says-who-asks-for-what" { } ''
      cp ${./tests/askpass.qml} askpass.qml
      cp ${./modules/quickshell/Askpass.js} Askpass.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml askpass.qml
      touch $out
    '';

  # The askpass asks a shell that restarted mid-prompt again, and refuses
  # once none has answered for a while, rather than holding ssh-tpm-agent
  # and every key behind it forever, as tests/askpass-waits.sh says. The
  # desktop test restarts a real shell, in minutes; this says it in one.
  the-askpass-outlives-its-shell =
    let
      askpass = (account graphical).home.sessionVariables.SSH_ASKPASS;
    in
    pkgs.runCommandLocal "the-askpass-outlives-its-shell" { } ''
      ${lib.getExe' pkgs.coreutils "timeout"} 300 ${lib.getExe pkgs.bash} \
        ${./tests/askpass-waits.sh} ${askpass} ||
        { echo "the askpass is still waiting, or got something wrong"; exit 1; }
      touch $out
    '';

  # The camera's viewfinder script names nothing it never set, which Lua
  # would otherwise read as nothing and carry on.
  the-camera-script-names-what-it-sets =
    pkgs.runCommandLocal "the-camera-script-names-what-it-sets"
      { nativeBuildInputs = [ pkgs.lua54Packages.luacheck ]; }
      ''
        luacheck --std lua54 --globals mp --no-max-line-length --no-color \
          ${./modules/camera/camera.lua}
        touch $out
      '';

  # firmware-settings names every setting the firmware holds apart from the
  # declared ones, writes them with the admin password from its input, and
  # wipes that password from the kernel whether the writes worked or not.
  # Files stand in for the kernel's firmware-attributes class.
  the-firmware-takes-what-is-declared =
    pkgs.runCommandLocal "the-firmware-takes-what-is-declared"
      { nativeBuildInputs = [ (pkgs.callPackage ./modules/firmware-settings/package.nix { }) ]; }
      ''
        fw=$PWD/fw/vendor
        setting() {
          mkdir -p "$fw/attributes/$1"
          echo "$2" > "$fw/attributes/$1/current_value"
        }
        value() { cat "$fw/attributes/$1/current_value"; }
        fail() { echo "$1"; cat out; exit 1; }
        mkdir -p "$fw/authentication/Admin" "$fw/attributes"
        echo 1 > "$fw/authentication/Admin/is_enabled"
        : > "$fw/authentication/Admin/current_password"
        echo 0 > "$fw/attributes/pending_reboot"
        setting Mode Adaptive
        setting Start 50
        setting Stop 90
        printf 'Mode=Custom\nStart=75\nStop=80\n' > declared
        export FIRMWARE_ATTRIBUTES=$PWD/fw FIRMWARE_SETTINGS=$PWD/declared
        password() { tr -d '\n' < "$fw/authentication/Admin/current_password"; }

        firmware-settings check > out 2>&1 && fail "check passed with settings to change"
        grep -qF "Start is 50, declared 75" out || fail "check didn't name what differs"

        echo secret | firmware-settings apply > out 2>&1 || fail "apply failed"
        [ "$(value Mode)/$(value Start)/$(value Stop)" = Custom/75/80 ] ||
          fail "apply left $(value Mode)/$(value Start)/$(value Stop)"
        [ -z "$(password)" ] || fail "apply left the password in the kernel"
        firmware-settings check > out 2>&1 || fail "check failed after apply"

        setting Locked old
        chmod a-w "$fw/attributes/Locked/current_value"
        echo Locked=new >> declared
        echo secret | firmware-settings apply > out 2>&1 && fail "apply passed a setting it couldn't write"
        grep -qF "Locked is old, declared new" out || fail "apply didn't name what it couldn't write"
        [ -z "$(password)" ] || fail "a failed apply left the password in the kernel"

        echo 0 > "$fw/authentication/Admin/is_enabled"
        printf 'Mode=Standard\n' > declared
        firmware-settings apply < /dev/null > out 2>&1 || fail "apply asked for a password none is set for"
        [ "$(value Mode)" = Standard ] || fail "apply without a password didn't write"

        printf 'Missing=x\n' > declared
        status=0
        firmware-settings check > out 2>&1 || status=$?
        [ "$status" = 2 ] || fail "check of a setting the firmware lacks exited $status"
        grep -qF "no firmware setting Missing" out || fail "check didn't name the missing setting"
        touch $out
      '';

  # device-check names a device whose power change failed, one the machine
  # expects that's gone and a disk smartd heard a fault from, each once, as a
  # warning and a notification, and again when it's fixed, and exits 0
  # whatever it finds. Files stand in for sysfs, smartd's variables are set
  # by hand, and a script stands in for notify-send.
  devices-that-stop-working-are-named =
    pkgs.runCommandLocal "devices-that-stop-working-are-named"
      { nativeBuildInputs = [ (pkgs.callPackage ./modules/device-check/package.nix { }) ]; }
      ''
        root=$PWD/root
        fail() { echo "$1"; cat out; exit 1; }
        device() {
          mkdir -p "$root/sys/devices/$1/power"
          echo "$2" > "$root/sys/devices/$1/power/runtime_status"
        }
        device pci/0000:00:05.0 error
        mkdir -p "$root/sys/bus/pci/drivers/intel-ipu7"
        ln -s ../../../bus/pci/drivers/intel-ipu7 "$root/sys/devices/pci/0000:00:05.0/driver"
        device "pci/0000:00:1f.3/sof_sdw/Jack Out" suspended
        device pci/0000:00:14.0 active
        mkdir -p "$root/sys/class/ieee80211/phy1"
        printf 'Bluetooth=/sys/class/bluetooth/hci0\nWi-Fi=/sys/class/ieee80211/phy*\n' > expected
        printf '#!/bin/sh\necho "$*" >> %s/notified\n' "$PWD" > notify
        chmod +x notify
        export DEVICE_CHECK_ROOT=$root DEVICE_CHECK_EXPECTED=$PWD/expected
        export DEVICE_CHECK_STATE=$PWD/state DEVICE_CHECK_NOTIFY=$PWD/notify

        SMARTD_DEVICE=/dev/nvme0 SMARTD_FAILTYPE=Health \
          SMARTD_MESSAGE="Device: /dev/nvme0, Critical Warning (0x04): Reliability" \
          device-check-disk > out 2>&1 || fail "smartd's hook failed"
        [ ! -s out ] || fail "smartd's hook wrote to its output, which smartd reads as trouble"

        device-check 2> out || fail "it failed with devices to name"
        grep -qxF "<4>0000:00:05.0 (intel-ipu7) failed a power change and won't wake until the next boot" out ||
          fail "it didn't warn of the failed device"
        grep -qxF "<4>Bluetooth is missing" out || fail "it didn't warn of the missing device"
        grep -qxF "<4>The disk /dev/nvme0 reports Critical Warning (0x04): Reliability" out ||
          fail "it didn't warn of the disk's fault"
        [ "$(wc -l < out)" = 3 ] || fail "it named more than what's wrong"
        [ "$(grep -c '^-u critical -a Devices ' notified)" = 3 ] || fail "it didn't notify of all three"

        : > notified
        device-check 2> out || fail "a second look failed"
        [ ! -s out ] && [ ! -s notified ] || fail "it named the same findings again"

        SMARTD_DEVICE=/dev/nvme0 SMARTD_FAILTYPE=Health \
          SMARTD_MESSAGE="Device: /dev/nvme0, Critical Warning (0x04): Reliability" \
          device-check-disk > out 2>&1 || fail "smartd's hook failed a second time"
        device-check 2> out || fail "a look after smartd's repeat failed"
        [ ! -s out ] && [ ! -s notified ] || fail "it named the disk's fault again when smartd repeated it"

        echo active > "$root/sys/devices/pci/0000:00:05.0/power/runtime_status"
        mkdir -p "$root/sys/class/bluetooth/hci0"
        rm "$root"/run/device-check/disks/*
        device-check 2> out || fail "a look after the fix failed"
        grep -qxF "<5>0000:00:05.0 (intel-ipu7) wakes again" out || fail "it didn't say the device wakes"
        grep -qxF "<5>Bluetooth is back" out || fail "it didn't say the device is back"
        grep -qxF "<5>The disk /dev/nvme0 no longer reports Critical Warning (0x04): Reliability" out ||
          fail "it didn't say the disk's fault cleared"
        [ "$(grep -c '^-u normal -a Devices ' notified)" = 3 ] || fail "it didn't notify of the fixes"
        touch $out
      '';

  # A workstation hears from its disks before they fail: smartd reads each
  # disk's own warnings and hands them to device-check, which names them to
  # whoever is signed in, rather than writing to every terminal.
  disks-say-when-they-fail =
    let
      inherit (workstation.config.services) smartd;
    in
    holds "disks-say-when-they-fail"
      "A workstation doesn't watch its disks, or doesn't hand what they report to device-check."
      (
        smartd.enable
        && smartd.autodetect
        && !smartd.notifications.wall.enable
        && lib.hasInfix "-H -f -l error " smartd.defaults.autodetected
        && lib.hasSuffix "/bin/device-check-disk" smartd.defaults.autodetected
      );

  # A machine with no disk that speaks SMART, as a virtual machine or a USB
  # stick can be, runs smartd without it failing: it finds nothing to watch
  # and stops cleanly. The build's sandbox has no disks, so the workstation's
  # own smartd command runs there as it would on such a machine.
  smartd-stops-cleanly-without-disks =
    let
      smartd = workstation.config.systemd.services.smartd.serviceConfig.ExecStart;
    in
    pkgs.runCommandLocal "smartd-stops-cleanly-without-disks"
      { nativeBuildInputs = [ pkgs.coreutils ]; }
      ''
        fail() { echo "$1"; cat out; exit 1; }
        status=0
        timeout 60 ${smartd} > out 2>&1 || status=$?
        [ "$status" != 124 ] || fail "smartd kept running with no disks to watch"
        [ "$status" = 0 ] || fail "smartd failed with no disks to watch, exiting $status"
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

  # The disk piece reads the guards' status files as tests/disk.qml writes
  # them, through Quickshell itself, with niri stood in for.
  the-disk-piece-reads-the-guards =
    let
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
            property var spawned: []
            function spawn(command) { spawned = command; }
        }
      '';
    in
    pkgs.runCommandLocal "the-disk-piece-reads-the-guards" { } ''
      mkdir runtime && chmod 700 runtime
      cp ${./modules/quickshell/pieces/Piece.qml} Piece.qml
      cp ${./modules/quickshell/pieces/Disk.qml} Disk.qml
      cp ${./tests/disk.qml} shell.qml
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      timeout 60 ${lib.getExe' pkgs.quickshell "qs"} -p shell.qml
      touch $out
    '';

  # The AI spend piece and the launcher's collection of it read what
  # spend-watch writes, from a day of a session's turns on two models, and the states
  # tests/spend.qml writes by hand, through Quickshell itself, with niri, do
  # not disturb and the shortcuts stood in for.
  the-shell-reads-the-spend-watch =
    let
      watch = pkgs.callPackage ./modules/spend-watch/package.nix { };
      singleton =
        name: body:
        pkgs.writeText name ''
          pragma Singleton
          import Quickshell
          import QtQuick
          Singleton { ${body} }
        '';
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = command; }
      '';
      shortcuts = singleton "Shortcuts.qml" ''
        property var actions: [{ opens: "spend", chord: [{ text: "Super" }, { text: "U" }] }]
      '';
      turn =
        ''{"type":"message","timestamp":"%s","message":{"role":"assistant","model":"%s",''
        + ''"usage":{"input":10,"output":100,"cacheRead":1000,"cacheWrite":100,''
        + ''"cost":{"total":1.5}}}}\n'';
    in
    pkgs.runCommandLocal "the-shell-reads-the-spend-watch" { } ''
      mkdir -p runtime/written sessions/thing state launcher && chmod 700 runtime
      log=sessions/thing/session.jsonl
      now=$(date -u +%s)
      at() { date -u -d @$((now - $1)) +%FT%TZ; }
      printf '{"type":"session","version":3,"id":"s1","timestamp":"%s","cwd":"/src/thing"}\n' \
        "$(at 90000)" > $log
      for hours in $(seq 24 -1 0); do
        printf '${turn}' "$(at $((hours * 3600)))" opus >> $log
        printf '${turn}' "$(at $((hours * 3600 + 60)))" sonnet >> $log
      done
      cat > config.json <<EOF
      {"sessions": "$PWD/sessions", "runs": "$PWD/runs.db", "registry": "$PWD/registry",
       "quests": "$PWD/quests", "state": "$PWD/state", "runtime": "$PWD/runtime/written",
       "zone": "UTC", "notices": {"enable": false}}
      EOF
      ${lib.getExe watch} --config config.json import
      ${lib.getExe watch} --config config.json show > runtime/written/status.json
      ${lib.getExe watch} --config config.json show collection > runtime/written/collection.json
      cp ${./modules/quickshell/pieces/Piece.qml} Piece.qml
      cp ${./modules/quickshell/pieces/Spend.qml} Spend.qml
      cp ${./modules/quickshell/Fuzzy.js} Fuzzy.js
      cp ${./modules/quickshell/launcher/Provider.qml} launcher/Provider.qml
      cp ${./modules/quickshell/launcher/Asking.js} launcher/Asking.js
      cp ${./modules/quickshell/launcher/Spend.qml} launcher/Spend.qml
      cp ${./tests/spend.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${singleton "Quiet.qml" "property bool sharing: false"} Quiet.qml
      cp ${shortcuts} Shortcuts.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      timeout 60 ${lib.getExe' pkgs.quickshell "qs"} -p shell.qml
      touch $out
    '';

  # A palette's row and its field keep their words apart and inside their
  # edges however long they run, ending what doesn't fit in an ellipsis, as
  # tests/rows.qml draws them through Quickshell in the theme's own face and
  # sizes, at a panel's width.
  palette-rows-keep-their-words-apart =
    let
      machine = graphical.config;
      lights = pkgs.writeText "Lights.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton { property bool caps: false }
      '';
    in
    pkgs.runCommandLocal "palette-rows-keep-their-words-apart" { } ''
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./modules/quickshell/Choice.qml} Choice.qml
      cp ${./modules/quickshell/Prompt.qml} Prompt.qml
      cp ${lights} Lights.qml
      cp ${./tests/rows.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = machine.fonts.packages; }}
      timeout 60 ${lib.getExe' pkgs.quickshell "qs"} -p shell.qml
      touch $out
    '';

  # The launcher's Keyboard collection finds the ErgoDox's keys and actions
  # and shows what it finds on the layout, as tests/keyboard.qml drives it
  # through Quickshell itself, over the firmware's own description of the
  # board, locked on Numb, with niri and the shortcuts stood in for.
  the-launcher-finds-the-keyboards-keys =
    let
      singleton =
        name: body:
        pkgs.writeText name ''
          pragma Singleton
          import Quickshell
          import QtQuick
          Singleton { ${body} }
        '';
      firmware = import ./modules/ergodox/firmware.nix pkgs {
        firmware = self.inputs.qmk;
        inherit (self.inputs) lufa printf;
      };
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = command; }
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-keyboards-keys" { } ''
      mkdir -p runtime config/ergodox launcher && chmod 700 runtime
      echo "Numb locked" > runtime/keyboard-layer
      cp ${firmware}/shell.json config/ergodox/layout.json
      cp ${./tests/keyboard.qml} shell.qml
      cp ${./modules/quickshell/KeyboardLayout.qml} KeyboardLayout.qml
      cp ${./modules/quickshell/KeyboardLayer.qml} KeyboardLayer.qml
      cp ${./modules/quickshell/Fuzzy.js} Fuzzy.js
      cp ${./modules/quickshell/launcher/Provider.qml} launcher/Provider.qml
      cp ${./modules/quickshell/launcher/Asking.js} launcher/Asking.js
      cp ${./modules/quickshell/launcher/Keyboard.qml} launcher/Keyboard.qml
      cp ${niri} Niri.qml
      cp ${singleton "Shortcuts.qml" "property var actions: []"} Shortcuts.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime XDG_CONFIG_HOME=$TMPDIR/config
      export LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
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
        cp ${./modules/quickshell/launcher/Asking.js} launcher/Asking.js
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

  # The music is the player that matters: of those playing, the last to
  # start, else the last that played, which a play key resumes, and never
  # playerctld's echo of another, as tests/music.qml lists them, with where
  # in the track said as the card says it.
  the-music-follows-the-player =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-music-follows-the-player" { } ''
      cp ${./tests/music.qml} music.qml
      cp ${./modules/quickshell/Music.js} Music.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml music.qml
      touch $out
    '';

  # Whether Joel's in a Tuple call comes from the daemon's log, which only a
  # real call writes, so tests/tuple.qml gives Tuple.js lines like its own:
  # what starts a call, what ends one, when it began, and a call's link.
  the-shell-follows-tuple-calls =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-shell-follows-tuple-calls" { } ''
      cp ${./tests/tuple.qml} tuple.qml
      cp ${./modules/tuple/Tuple.js} Tuple.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml tuple.qml
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
  # ctrl+shift+t is Neovim's Ctrl+Shift+T, and its super+return is niri's
  # Super+Return. A sequence whose first key a layer above takes can never
  # start, so it's refused too, as is a layer claiming one chord twice. Neovim
  # and pi are never heard at once, so a chord both claim is no clash.
  a-claimed-chord-is-refused =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        {
          home-manager.users.someone.jitsusama.keys.neovim = [
            {
              chord = "Ctrl+Shift+T";
              does = "Reopen a closed buffer";
            }
            {
              chord = "Ctrl+Shift+H g";
              does = "Go to the help";
            }
            {
              chord = "Ctrl+G";
              does = "Say where the file is";
            }
          ];
          home-manager.users.someone.jitsusama.kitty.keys = lib.mkAfter [
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
        && lib.hasInfix "ctrl+shift+t: terminal (Open a tab here) takes it from neovim (Reopen a closed buffer)" refusals
        && lib.hasInfix "ctrl+shift+h: terminal (Pick a word from the screen) takes it from neovim (Go to the help)" refusals
        && !lib.hasInfix "(Write the message in Neovim)" refusals
        && lib.hasInfix "super+enter: desktop (Open a terminal) takes it from terminal (Open a tab)" refusals
        && lib.hasInfix "ctrl+shift+c in terminal: Copy, Copy again" refusals
      );

  # The boot's lock is drawn whole: its picture at each scale holds all the
  # ink the glyph has when drawn with room to spare, and is the line's height.
  boot-lock-is-whole =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { jitsusama.login.account = "someone"; }
      ];
      inherit (machine.config.boot.plymouth) themePackages;
      theme = (account machine).jitsusama.theme;
    in
    pkgs.runCommandLocal "boot-lock-is-whole"
      {
        nativeBuildInputs = [
          pkgs.imagemagick
          pkgs.pango.bin
        ];
        LC_ALL = "C.UTF-8";
        FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = machine.config.fonts.packages; };
      }
      ''
        export HOME=$PWD
        dir=${lib.head themePackages}/share/plymouth/themes/jitsusama
        for scale in 1 2; do
          pango-view -q --no-display --dpi=$((96 * scale)) --margin=$((64 * scale)) \
            --antialias=gray --hinting=none --background=transparent --foreground=white \
            --font=${lib.escapeShellArg "${theme.font.mono.family} ${toString theme.type.mono.large}"} \
            --output=roomy.png --text=$'\uf023'
          pango-view -q --no-display --dpi=$((96 * scale)) --margin=0 \
            --hinting=none --font=${lib.escapeShellArg "${theme.font.mono.family} ${toString theme.type.mono.large}"} \
            --output=line.png --text=x
          want="$(magick roomy.png -trim -format '%wx%h' info:)"
          got="$(magick $dir/lock-$scale.png -trim -format '%wx%h' info:)"
          if [ "$got" != "$want" ]; then
            echo "the lock drawn at $scale holds $got of its $want ink"; exit 1
          fi
          line="$(magick line.png -format '%h' info:)"
          height="$(magick $dir/lock-$scale.png -format '%h' info:)"
          if [ "$height" != "$line" ]; then
            echo "the lock drawn at $scale is $height tall, not the line's $line"; exit 1
          fi
        done
        touch $out
      '';

  # A failed boot's shell says what stopped it, as tests/boot-stopped.sh
  # says, with stand-ins for systemctl and journalctl. The boot test shows it
  # on a real console, in minutes; this holds what it says, in seconds.
  a-failed-boot-says-what-stopped-it =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { jitsusama.login.account = "someone"; }
      ];
      stopped = lib.removePrefix "-" (
        lib.findFirst (lib.hasInfix "boot-stopped") (throw "the emergency shell runs no boot-stopped")
          machine.config.systemd.services.emergency.serviceConfig.ExecStartPre
      );
    in
    pkgs.runCommandLocal "a-failed-boot-says-what-stopped-it" { } ''
      ${lib.getExe pkgs.bash} ${./tests/boot-stopped.sh} ${stopped}
      touch $out
    '';

  # The console wears the theme of the account the machine signs in, in the
  # console's order, and follows it when the theme changes. A machine that
  # names no account keeps the kernel's colours.
  the-console-wears-the-theme =
    let
      wearing =
        theme:
        (bareNixos [
          self.nixosModules.workstation
          self.nixosModules.graphical
          {
            jitsusama.login.account = "someone";
            home-manager.users.someone.jitsusama.theme.name = theme;
          }
        ]).config.console.colors;
      file = theme: builtins.fromTOML (builtins.readFile ./modules/theme/${theme}.toml);
      pigment =
        theme: role:
        lib.toLower (lib.removePrefix "#" (file theme).palette.${(file theme).terminal.${role}});
      follows =
        theme:
        let
          colours = map lib.toLower (wearing theme);
        in
        lib.length colours == 16
        && lib.elemAt colours 0 == pigment theme "black"
        && lib.elemAt colours 1 == pigment theme "red"
        && lib.elemAt colours 15 == pigment theme "bright_white";
    in
    holds "the-console-wears-the-theme"
      "The console's colours aren't the account's theme, in the console's order."
      (follows "osaka-jade" && follows "flexoki-light" && graphical.config.console.colors == [ ]);

  # The console's font is the account's monospace face at Medium, drawn into
  # the kernel's 16 by 32 cell: a PSF 2 font of at most 256 glyphs, so the
  # console keeps its bright colours, naming what each glyph stands for, with
  # slot 32 blank, since the console clears the screen with it and kbd
  # refuses a font without, and loaded in the initrd. A machine that names
  # no account keeps the kernel's own font.
  the-console-wears-the-face =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { jitsusama.login.account = "someone"; }
      ];
      inherit (machine.config) console;
      face = (account machine).jitsusama.theme.font.mono.family;
    in
    if !console.earlySetup || graphical.config.console.font != null then
      holds "the-console-wears-the-face"
        "The console's font isn't loaded in the initrd, or a machine without an account has one."
        false
    else
      pkgs.runCommandLocal "the-console-wears-the-face" { } ''
        font=${console.font}
        word() { od -An -t u4 -j $(( $1 * 4 )) -N 4 "$font" | tr -d ' '; }
        fail() { echo "The console's font $1."; exit 1; }
        [ "$(word 0)" = 2253043058 ] || fail "isn't a PSF 2 font"
        [ "$(word 3)" = 1 ] || fail "doesn't say what its glyphs stand for"
        [ "$(word 4)" -le 256 ] || fail "holds $(word 4) glyphs, past which the console loses its bright colours"
        [ "$(word 6)x$(word 7)" = 32x16 ] || fail "draws $(word 7) by $(word 6), not the kernel's 16 by 32"
        ink=$(od -An -t x1 -v -j $(( 32 + 32 * $(word 5) )) -N "$(word 5)" "$font" | tr -d ' \n0')
        [ -z "$ink" ] || fail "has ink in slot 32, which the console clears the screen with"
        printf '%s\nMedium\n' ${lib.escapeShellArg face} | cmp -s - "$(dirname "$font")/face" \
          || fail "isn't drawn from ${face} at Medium"
        tail -c +$(( 32 + $(word 4) * $(word 5) + 1 )) "$font" | tr '\377' '\n' > table
        for c in A '~' ─ █ ● é; do
          grep -qxF "$c" table || fail "has no glyph for $c"
        done
        touch $out
      '';

  # The panic screen wears the theme of the account the machine signs in, its
  # text on its background, and follows it when the theme changes. A machine
  # that names no account keeps the kernel's white on black. Read from the
  # kernel's settings, so no kernel is built to check it.
  the-panic-screen-wears-the-theme =
    let
      kernelOf = modules: (bareNixos modules).config.boot.kernelPackages.kernel.structuredExtraConfig;
      wearing =
        theme:
        kernelOf [
          self.nixosModules.workstation
          self.nixosModules.graphical
          self.nixosModules.kernel
          {
            jitsusama.login.account = "someone";
            home-manager.users.someone.jitsusama.theme.name = theme;
          }
        ];
      file = theme: builtins.fromTOML (builtins.readFile ./modules/theme/${theme}.toml);
      pigment =
        theme: role:
        "0x${lib.toLower (lib.removePrefix "#" (file theme).palette.${(file theme).roles.${role}})}";
      colour =
        settings: option: (settings.${option}.content or settings.${option} or { }).freeform or null;
      follows =
        theme:
        colour (wearing theme) "DRM_PANIC_FOREGROUND_COLOR" == pigment theme "text"
        && colour (wearing theme) "DRM_PANIC_BACKGROUND_COLOR" == pigment theme "background";
      bare = kernelOf [
        self.nixosModules.workstation
        self.nixosModules.graphical
        self.nixosModules.kernel
      ];
    in
    holds "the-panic-screen-wears-the-theme"
      "The panic screen's colours aren't the account's theme, text on background."
      (follows "osaka-jade" && follows "flexoki-light" && !(bare ? DRM_PANIC_FOREGROUND_COLOR));

  # Picking another theme changes everything the account draws: the desktop
  # rendered in Flexoki Light keeps none of Osaka Jade's colours anywhere in
  # its files, so no surface was left behind in the old one. The design
  # document installed for agents beside the aesthetic skill is passed over:
  # it describes themes, Osaka Jade among them, and draws nothing.
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
        --glob '!**/skills/aesthetic/design.md' \
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

  # Btrfs finds a damaged block only when something reads it, so the disk is
  # scrubbed on a schedule, whole, from its root.
  disks-are-scrubbed =
    let
      machine = bareNixos [
        self.nixosModules.disko
        {
          jitsusama.disk = {
            device = "/dev/disk/by-id/nvme-example";
            swapSize = "4G";
          };
        }
      ];
      inherit (machine.config.services.btrfs) autoScrub;
    in
    holds "disks-are-scrubbed" "The disk's Btrfs is never scrubbed, so damage stays hidden." (
      autoScrub.enable && autoScrub.fileSystems == [ "/" ]
    );

  # The kernel protects session.slice's memory only as far as every slice
  # above it is protected, so a floor one of them lacks is a floor nobody has,
  # and nothing says so. Only a hard floor counts: the kernel takes a soft one
  # back under pressure, and the compositor reads keys late.
  session-memory-is-protected =
    let
      inherit (graphical.config) systemd;
      floors = [
        systemd.user.slices.session.sliceConfig.MemoryMin or null
        systemd.slices.user.sliceConfig.MemoryMin or null
        systemd.slices."user-".sliceConfig.MemoryMin or null
        systemd.services."user@".serviceConfig.MemoryMin or null
      ];
    in
    holds "session-memory-is-protected"
      "A slice above session.slice lacks its hard memory floor, so session.slice has none."
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

  # Nix collects garbage during a build once free space falls under min-free,
  # and stops at max-free. A max-free at or under min-free stops it before it
  # starts, and nothing says so.
  nix-collects-before-the-disk-fills =
    let
      inherit (workstation.config.nix.settings) min-free max-free;
    in
    holds "nix-collects-before-the-disk-fills"
      "The Nix daemon never collects garbage during a build: min-free is ${toString min-free}, max-free ${toString max-free}."
      (min-free > 0 && max-free > min-free);

  # The disk guard's own memory ceiling never stops it, and the account's
  # guard yields to whatever the account runs. Past the ceiling the kernel
  # kills in the unit, and systemd's default then stops the whole unit, a
  # guard that was only waiting on a command among it.
  disk-guard-stays-in-the-background =
    let
      system = workstation.config.systemd.services.disk-guard.serviceConfig;
      user = (account workstation).systemd.user.services.disk-guard.Service;
    in
    holds "disk-guard-stays-in-the-background"
      "A command overrunning the disk guard's memory would stop the guard, or the account's guard runs outside background.slice."
      (
        system.OOMPolicy == "continue" && user.OOMPolicy == "continue" && user.Slice == "background.slice"
      );

  # spend-watch only watches, so it yields to whatever the account runs, and
  # comes back when it fails.
  spend-watch-stays-in-the-background =
    let
      service = (account workstation).systemd.user.services.spend-watch.Service;
    in
    holds "spend-watch-stays-in-the-background"
      "spend-watch runs outside background.slice, isn't restarted when it fails, or takes its status with it when it stops."
      (
        service.Slice == "background.slice"
        && service.Restart == "on-failure"
        && service.RuntimeDirectoryPreserve == "yes"
      );

  # A Mac's guards run under launchd, the machine's as a daemon and the
  # account's as an agent in the background, with a Mac's own paths and
  # nothing to confine a linking run to, since a Mac has no scopes.
  the-mac-guards-its-disk =
    let
      daemon = macWorkstation.config.launchd.daemons.disk-guard.serviceConfig;
      agent = (account macWorkstation).launchd.agents.disk-guard;
      system = macWorkstation.config.jitsusama.disk-guard.settings;
      user = (account macWorkstation).jitsusama.disk-guard.settings;
      # Homebrew comes with the graphical role.
      brewing =
        (account (bareDarwin [
          self.darwinModules.workstation
          self.darwinModules.graphical
        ])).jitsusama.disk-guard.settings.sources;
    in
    holds "the-mac-guards-its-disk"
      "A Mac's guards don't run under launchd, their config names Linux's paths or systemd-run, or a Mac's own sources are missing."
      (
        builtins.elemAt daemon.ProgramArguments 1 == "system"
        && daemon.KeepAlive
        && agent.enable
        && builtins.elemAt agent.config.ProgramArguments 1 == "user"
        && agent.config.ProcessType == "Background"
        && system.dedupe.systemd_run == null
        && system.sources.tmp.path == "/private/tmp"
        && user.sources.trash.path == "/Users/someone/.Trash"
        && user.sources.derived-data.kind == "cache"
        && !(user.sources ? homebrew)
        && builtins.elemAt brewing.homebrew.command 1 == "cleanup"
        && !((account macWorkstation).systemd.user.services ? disk-guard)
      );

  # greetd waits for the GPU the hardware names rather than for the boot to go
  # idle, and the card gets the device unit it waits on. Without the udev tag
  # the unit never appears, and the login waits out the device timeout.
  # Session machines join virbr0 without a password, which needs the bridge
  # helper to allow it and the default network to be up from boot.
  session-machines-have-a-bridge =
    let
      machine = bareNixos [ self.nixosModules.libvirt ];
      inherit (machine.config) systemd virtualisation;
      network = systemd.services.libvirt-default-network;
    in
    holds "session-machines-have-a-bridge"
      "Session machines can't join virbr0: the bridge helper doesn't allow it, or nothing starts libvirt's default network at boot."
      (
        lib.elem "virbr0" virtualisation.libvirtd.allowedBridges
        && lib.elem "multi-user.target" network.wantedBy
        && lib.hasInfix "net-start default" network.script
      );

  # A development DNS server answers for its own domains through a file in
  # resolved.conf.d, so resolved's global settings stay as they ship: a
  # server, domain, DNSSEC or DNS over TLS set there would reach it as well.
  # What a network needs goes in its own .network file. The machine checked
  # has networkd without the laptop role, whose keyboard and power modules
  # leave resolved alone, so it holds whichever networking the role takes.
  resolved-is-set-per-network =
    let
      laptop = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        self.nixosModules.systemd-networkd
      ];
      resolve = laptop.config.services.resolved.settings.Resolve;
    in
    holds "resolved-is-set-per-network"
      "A machine with networkd sets resolved's global servers, domains, DNSSEC or DNS over TLS."
      (
        laptop.config.services.resolved.enable
        && resolve.DNS == [ ]
        && resolve.Domains == [ ]
        && resolve.DNSSEC == false
        && resolve.DNSOverTLS == false
      );

  # UPower waits 20 s before its action, and under load 1% of a laptop's
  # battery lasts about a minute, so acting at 2% acts after it's flat.
  batteries-leave-time-to-power-off =
    let
      inherit ((bareNixos [ self.nixosModules.power-profiles-daemon ]).config.services) upower;
    in
    holds "batteries-leave-time-to-power-off"
      "UPower acts below 5% or by sleeping, so a laptop under load goes flat first."
      (upower.enable && upower.percentageAction >= 5 && upower.criticalPowerAction == "PowerOff");

  # The profile follows the charger: performance on mains, balanced on
  # battery, and power saver once the battery falls below UPower's low mark,
  # staying there while the battery reads above it again, until the charger is
  # back. It acts only when that choice changes, so a profile picked by hand
  # holds, and a line from UPower that isn't a change does nothing. UPower and
  # the daemon are stand-ins: each time the service asks whether the machine
  # is on battery, UPower answers with the next of the states listed below.
  the-profile-saves-power-on-a-low-battery =
    let
      inherit ((bareNixos [ self.nixosModules.power-profiles-daemon ]).config) services systemd;
      unit = systemd.services.power-profile-follows-the-charger;
    in
    pkgs.runCommandLocal "the-profile-saves-power-on-a-low-battery" { } ''
      ${lib.optionalString (services.upower.percentageLow != 20) ''
        echo "UPower's low mark isn't the 20% the steps below expect." >&2
        exit 1
      ''}
      export STATE=$PWD
      mkdir bin
      cat > steps <<'EOF'
      false 79
      true 35
      true 20
      true 19
      true 21
      false 21
      true 15
      false 15
      EOF
      cat > bin/busctl <<'EOF'
      #!${pkgs.bash}/bin/bash
      case "$*" in
        *OnBattery*)
          if [ ! -s "$STATE/steps" ]; then
            echo "asked once too often" >> "$STATE/errors"
            exit 1
          fi
          head -n 1 "$STATE/steps" > "$STATE/now"
          tail -n +2 "$STATE/steps" > "$STATE/rest"
          mv "$STATE/rest" "$STATE/steps"
          read -r battery _ < "$STATE/now"
          echo "b $battery"
          ;;
        *Percentage*)
          read -r _ percent < "$STATE/now"
          echo "d $percent.6"
          ;;
      esac
      EOF
      cat > bin/powerprofilesctl <<'EOF'
      #!${pkgs.bash}/bin/bash
      case "$1" in
        list) printf '%s\n' '  performance:' '* balanced:' '  power-saver:' ;;
        set) echo "$(cat "$STATE/now"): $2" >> "$STATE/calls" ;;
      esac
      EOF
      cat > bin/gdbus <<'EOF'
      #!${pkgs.bash}/bin/bash
      changed="org.freedesktop.DBus.Properties.PropertiesChanged"
      echo "The name org.freedesktop.UPower is owned by :1.12"
      for _ in 1 2 3 4 5 6 7; do
        echo "/org/freedesktop/UPower/devices/DisplayDevice: $changed ('org.freedesktop.UPower.Device', ...)"
      done
      EOF
      chmod +x bin/*
      export PATH=$PWD/bin:${
        lib.makeBinPath [
          pkgs.coreutils
          pkgs.diffutils
          pkgs.gnugrep
        ]
      }
      if ${unit.serviceConfig.ExecStart or "false"}; then
        echo "The service ended without its monitor failing." >&2
        exit 1
      fi
      cat > expected <<'EOF'
      false 79: performance
      true 35: balanced
      true 19: power-saver
      false 21: performance
      true 15: power-saver
      false 15: performance
      EOF
      if [ -e errors ] || [ -s steps ] || ! diff expected calls; then
        echo "The power profile didn't follow the battery as listed." >&2
        exit 1
      fi
      touch $out
    '';

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

  # thermald reads the XPS 14's tables, sensors and the CPU's power limits
  # once, as it starts, and stops if its adaptive mode finds no zone, which
  # it did when udev loaded their drivers after it started.
  thermald-starts-after-its-drivers =
    let
      xps = bareNixos [ self.nixosModules.dell-xps-14-da14260 ];
      inherit (xps.config) boot systemd;
      early = [
        "intel_rapl_msr"
        "coretemp"
        "int3400_thermal"
        "int3403_thermal"
        "processor_thermal_device_pci"
      ];
    in
    holds "thermald-starts-after-its-drivers"
      "thermald can start before a driver it reads, or a restart can't run it adaptive again."
      (
        lib.all (module: lib.elem module boot.kernelModules) early
        && (systemd.services.thermald.serviceConfig.Restart or null) == "on-failure"
        && lib.hasInfix "ignore_adaptive" (systemd.services.thermald.serviceConfig.ExecStartPre or "")
      );

  # A machine that names a click strength sets it on each hidraw device as it
  # appears and again after every kind of sleep. One that names none runs
  # nothing.
  the-click-strength-holds-after-sleep =
    let
      machine =
        strength:
        (bareNixos [
          self.nixosModules.touchpad
          { jitsusama.touchpad.clickStrength = strength; }
        ]).config;
      set = machine 30;
      unset = machine null;
      unit = set.systemd.services.touchpad-click-strength or { };
      sleeps = [
        "suspend.target"
        "hibernate.target"
        "hybrid-sleep.target"
        "suspend-then-hibernate.target"
      ];
    in
    holds "the-click-strength-holds-after-sleep"
      "The touchpad isn't set to 30% as it appears and after each sleep, or it's set unasked."
      (
        lib.hasInfix "touchpad-click-strength 30 %k" set.services.udev.extraRules
        && lib.hasSuffix "touchpad-click-strength 30" (unit.serviceConfig.ExecStart or "")
        && lib.all (target: lib.elem target unit.wantedBy && lib.elem target unit.after) sleeps
        && !(unset.systemd.services ? touchpad-click-strength)
        && !(lib.hasInfix "touchpad-click-strength" unset.services.udev.extraRules)
      );

  # niri has no colour management, so a program that converts its colours
  # itself has to be told the panel's, or it draws sRGB stretched over it.
  chromium-knows-the-panels-colours =
    let
      xps = bareNixos [
        self.nixosModules.dell-xps-14-da14260
        { nixpkgs.config.allowUnfree = true; }
      ];
      flag = "--force-color-profile=${xps.config.jitsusama.panel.colourSpace or "none"}";
      told = package: lib.hasInfix flag (package.postFixup or "");
    in
    holds "chromium-knows-the-panels-colours"
      "Chrome, Slack or 1Password would draw sRGB stretched over the XPS 14's wider panel."
      (
        (xps.config.jitsusama.panel.colourSpace or null) == "display-p3-d65"
        && lib.all told [
          xps.pkgs.google-chrome
          xps.pkgs.slack
          xps.pkgs._1password-gui
        ]
      );

  # The XPS 14's panel stays fast at 30%, drops to 60 Hz once the battery falls
  # below it, stays there while the battery reads 31% again, and comes back to
  # its fastest rate on the charger. Only the panel's modes at the size it shows
  # count, and a line from UPower that isn't a change does nothing. UPower and
  # niri are stand-ins: each time the service asks whether the machine is on
  # battery, UPower answers with the next of the states listed below.
  the-panel-slows-on-a-low-battery =
    let
      xps = bareNixos [
        self.nixosModules.graphical
        self.nixosModules.dell-xps-14-da14260
      ];
      unit = xps.config.systemd.user.services.panel-refresh or null;
      started = unit != null && lib.elem "graphical-session.target" unit.wantedBy;
      mode = width: height: refresh_rate: { inherit width height refresh_rate; };
      outputs = builtins.toFile "outputs.json" (
        builtins.toJSON {
          HDMI-A-1 = {
            current_mode = 0;
            modes = [
              (mode 1920 1080 144000)
              (mode 1920 1080 60000)
            ];
          };
          eDP-1 = {
            current_mode = 0;
            modes = [
              (mode 2880 1800 120001)
              (mode 2880 1800 60000)
              (mode 2880 1800 48000)
              (mode 1920 1200 165000)
            ];
          };
        }
      );
    in
    pkgs.runCommandLocal "the-panel-slows-on-a-low-battery" { } ''
      ${lib.optionalString (!started) ''
        echo "The XPS 14 has no panel-refresh service started with the session." >&2
        exit 1
      ''}
      export STATE=$PWD
      mkdir bin
      cat > steps <<'EOF'
      false 79
      true 35
      true 30
      true 29
      true 31
      true 27
      false 27
      true 25
      EOF
      cat > bin/busctl <<'EOF'
      #!${pkgs.bash}/bin/bash
      case "$*" in
        *OnBattery*)
          if [ ! -s "$STATE/steps" ]; then
            echo "asked once too often" >> "$STATE/errors"
            exit 1
          fi
          head -n 1 "$STATE/steps" > "$STATE/now"
          tail -n +2 "$STATE/steps" > "$STATE/rest"
          mv "$STATE/rest" "$STATE/steps"
          read -r battery _ < "$STATE/now"
          echo "b $battery"
          ;;
        *Percentage*)
          read -r _ percent < "$STATE/now"
          echo "d $percent.6"
          ;;
      esac
      EOF
      cat > bin/niri <<'EOF'
      #!${pkgs.bash}/bin/bash
      case "$1 $2" in
        "msg --json") cat ${outputs} ;;
        "msg output") echo "$(cat "$STATE/now"): $3 $4 $5" >> "$STATE/calls" ;;
      esac
      EOF
      cat > bin/gdbus <<'EOF'
      #!${pkgs.bash}/bin/bash
      changed="org.freedesktop.DBus.Properties.PropertiesChanged"
      echo "The name org.freedesktop.UPower is owned by :1.12"
      for _ in 1 2 3 4 5 6 7; do
        echo "/org/freedesktop/UPower/devices/DisplayDevice: $changed ('org.freedesktop.UPower.Device', ...)"
      done
      EOF
      chmod +x bin/*
      export PATH=$PWD/bin:${
        lib.makeBinPath [
          pkgs.coreutils
          pkgs.diffutils
          pkgs.gnugrep
          pkgs.jq
        ]
      }
      if ${unit.serviceConfig.ExecStart or "false"}; then
        echo "The service ended without its monitor failing." >&2
        exit 1
      fi
      cat > expected <<'EOF'
      true 29: eDP-1 mode 2880x1800@60.000
      false 27: eDP-1 mode 2880x1800@120.001
      true 25: eDP-1 mode 2880x1800@60.000
      EOF
      if [ -e errors ] || [ -s steps ] || ! diff expected calls; then
        echo "The panel's rate didn't follow the battery as listed." >&2
        exit 1
      fi
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

    # And with an account's theme, whose panic colours the kernel has to take.
    kernel-settings-hold-with-a-theme =
      (bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        self.nixosModules.kernel
        {
          jitsusama.login.account = "someone";
          home-manager.users.someone.jitsusama.theme.name = "osaka-jade";
        }
      ]).config.boot.kernelPackages.kernel;
  }
  // {
    # The SoundWire machine driver binds each amplifier by the names of the
    # DAIs it lists for it, and one the amplifiers' driver doesn't register
    # leaves no sound card at all. 7.2.11 named the OT25 DAI before it had
    # one. Read from the source the XPS 14 builds, with its patches.
    speaker-amps-have-every-dai-named =
      let
        inherit ((bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.boot.kernelPackages)
          kernel
          ;
        codec = "sound/soc/codecs/cs35l56.c";
        machine = "sound/soc/sdw_utils/soc_sdw_utils.c";
        patches = lib.filter (patch: patch != null) (map (patch: patch.patch) kernel.kernelPatches);
      in
      pkgs.runCommandLocal "speaker-amps-have-every-dai-named"
        { nativeBuildInputs = [ pkgs.patchutils ]; }
        ''
          tar xf ${kernel.src} --strip-components=1 --wildcards '*/${codec}' '*/${machine}'
          for patch in ${lib.escapeShellArgs (map (patch: "${patch}") patches)}; do
            filterdiff -p1 -i ${codec} -i ${machine} "$patch" | patch -p1 --forward --batch --silent
          done
          named=$(
            { grep -o '\.dai_name = "cs35l56-[^"]*"' ${machine} || true; } |
              cut -d '"' -f 2 | sort -u
          )
          if [ -z "$named" ]; then
            echo "The SoundWire machine driver names no cs35l56 DAI, so there's nothing to hold."
            exit 1
          fi
          missing=
          for dai in $named; do
            grep -qF ".name = \"$dai\"" ${codec} || missing="$missing $dai"
          done
          if [ -n "$missing" ]; then
            echo "The SoundWire machine driver names DAIs no amplifier registers:$missing"
            exit 1
          fi
          touch $out
        '';

    # The legacy profile file, the one power-profiles-daemon writes, offers
    # only what every profile driver takes, so a profile Dell's driver offers
    # that the SoC slider doesn't never reaches either. Without quiet there,
    # power-saver runs as balanced. Read from the XPS 14's patched source.
    platform-profiles-reach-the-legacy-file =
      let
        inherit ((bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.boot.kernelPackages)
          kernel
          ;
        dell = "drivers/platform/x86/dell/dell-pc.c";
        slider = "drivers/thermal/intel/int340x_thermal/processor_thermal_soc_slider.c";
        patches = lib.filter (patch: patch != null) (map (patch: patch.patch) kernel.kernelPatches);
      in
      pkgs.runCommandLocal "platform-profiles-reach-the-legacy-file"
        { nativeBuildInputs = [ pkgs.patchutils ]; }
        ''
          tar xf ${kernel.src} --strip-components=1 --wildcards '*/${dell}' '*/${slider}'
          for patch in ${lib.escapeShellArgs (map (patch: "${patch}") patches)}; do
            filterdiff -p1 -i ${dell} -i ${slider} "$patch" | patch -p1 --forward --batch --silent
          done
          profiles() {
            { grep -o 'set_bit(PLATFORM_PROFILE_[A-Z_]*' "$1" || true; } | cut -d '(' -f 2 | sort -u
          }
          offered=$(profiles ${dell})
          if [ -z "$offered" ]; then
            echo "Dell's profile driver offers no profile, so there's nothing to hold."
            exit 1
          fi
          taken=$(profiles ${slider})
          missing=
          for profile in $offered; do
            echo "$taken" | grep -qx "$profile" || missing="$missing $profile"
          done
          if [ -n "$missing" ]; then
            echo "The SoC slider doesn't take profiles Dell's driver offers:$missing"
            exit 1
          fi
          touch $out
        '';

    # The capture side of the XPS 14's IPU, and the ports its video nodes sit
    # in, are freed at the V4L2 device's last reference rather than with the
    # device, since a file open on one of its nodes outlives an unbind and
    # reaches them as it closes.
    ipu7-outlives-its-open-files =
      let
        inherit ((bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.boot.kernelPackages)
          kernel
          ;
        isys = "drivers/staging/media/ipu7/ipu7-isys.c";
        patches = lib.filter (patch: patch != null) (map (patch: patch.patch) kernel.kernelPatches);
      in
      pkgs.runCommandLocal "ipu7-outlives-its-open-files" { nativeBuildInputs = [ pkgs.patchutils ]; } ''
        tar xf ${kernel.src} --strip-components=1 --wildcards '*/${isys}'
        for patch in ${lib.escapeShellArgs (map (patch: "${patch}") patches)}; do
          filterdiff -p1 -i ${isys} "$patch" | patch -p1 --forward --batch --silent
        done
        fail() { echo "The IPU7's capture side $1, so a node still open after an unbind reads freed memory as it closes."; exit 1; }
        grep -q 'sizeof(\*isys)' ${isys} || fail "isn't allocated where this check looks"
        ! grep -q 'devm_kzalloc(&auxdev->dev, sizeof(\*isys)' ${isys} || fail "is freed with its device"
        ! grep -q 'devm_kcalloc' ${isys} || fail "has its CSI-2 ports freed with its device"
        grep -q 'v4l2_dev.release = ' ${isys} || fail "sets no release for its V4L2 device"
        touch $out
      '';

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
        cp ${files."niri/keyboard.kdl".source} niri/keyboard.kdl
        niri validate --config niri/config.kdl
        touch $out
      '';

    # niri's own screens are drawn in colours no theme reaches, so no key
    # opens one: not its screenshot screen, not its key list, and no quit that
    # asks first. The launcher's Log Out quits without asking too.
    niri-never-shows-its-own-screens =
      let
        actions = map (bind: bind.action) (account graphical).jitsusama.niri.binds;
        opens =
          action:
          lib.hasPrefix "screenshot;" action
          || lib.hasPrefix "screenshot " action
          || lib.hasPrefix "show-hotkey-overlay" action
          || (lib.hasPrefix "quit" action && !(lib.hasInfix "skip-confirmation=true" action));
        session = builtins.readFile ./modules/quickshell/launcher/Session.qml;
      in
      holds "niri-never-shows-its-own-screens"
        "A key opens one of niri's own screens, or Log Out asks niri's question: ${lib.concatStringsSep ", " (lib.filter opens actions)}"
        (!(lib.any opens actions) && lib.hasInfix "Quit: { skip_confirmation: true }" session);

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

    # kitty draws the monospaced face with the features the account's
    # fontconfig gives it: shaped through kitty's own font code, an equals
    # sign, a comparison and an arrow change their glyphs beside the face
    # alone, and the pipe and the range, whose sets are left off, don't.
    kitty-draws-the-ligatures =
      let
        home = account graphical;
        inherit (home.jitsusama.theme.font) mono;
        features = home.xdg.configFile."fontconfig/conf.d/90-hm-features.conf".source;
        conf =
          extra:
          pkgs.writeText "fonts.conf" ''
            <?xml version="1.0"?>
            <!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
            <fontconfig>
              <dir>${pkgs.nerd-fonts.monaspace}/share/fonts</dir>
              <cachedir>cache</cachedir>
              ${extra}
            </fontconfig>
          '';
        shape = pkgs.writeText "shape.py" ''
          import sys
          from kitty.fonts.render import shape_string
          text = sys.argv[-1]
          print(" ".join(str(glyphs) for *_, glyphs in shape_string(text, family="${mono.family}")))
        '';
      in
      pkgs.runCommandLocal "kitty-draws-the-ligatures" { nativeBuildInputs = [ pkgs.kitty ]; } ''
        export HOME=$PWD
        shaped() {
          FONTCONFIG_FILE=$1 kitty +runpy "exec(open('${shape}').read())" "$2"
        }
        for text in '!=' '>=' '->'; do
          if [ "$(shaped ${conf ""} "$text")" = "$(shaped ${conf "<include>${features}</include>"} "$text")" ]; then
            echo "$text is drawn without its ligature"
            exit 1
          fi
        done
        for text in '|>' '1..=2'; do
          if [ "$(shaped ${conf ""} "$text")" != "$(shaped ${conf "<include>${features}</include>"} "$text")" ]; then
            echo "$text is drawn as a ligature, whose set is meant to be off"
            exit 1
          fi
        done
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

    # mpv plays a moment of video and sound with the account's settings and
    # the controller's, saying nothing about either: mpv names an option it
    # doesn't know and goes on without it, and the controller does the same
    # for a key, so a setting a new release drops would vanish without a
    # word. A video file opens in mpv.
    mpv-accepts-its-configuration =
      let
        home = account graphical;
        mpv = lib.findFirst (package: lib.hasPrefix "mpv" (lib.getName package)) null home.home.packages;
        files = home.xdg.configFile;
        opens = graphical.config.xdg.mime.defaultApplications."video/mp4" or "";
      in
      assert opens == "mpv.desktop";
      pkgs.runCommandLocal "mpv-accepts-its-configuration" { nativeBuildInputs = [ mpv ]; } ''
        export HOME=$PWD FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        mkdir -p mpv/script-opts
        cp ${files."mpv/mpv.conf".source} mpv/mpv.conf
        cp ${files."mpv/script-opts/osc.conf".source} mpv/script-opts/osc.conf
        # Kept open, mpv would hold the last frame and never finish.
        mpv --config-dir=$PWD/mpv --vo=null --ao=null --force-window=no --keep-open=no \
          --save-position-on-quit=no --frames=10 \
          --script-opts-append=osc-visibility=always \
          'av://lavfi:testsrc=duration=1:size=64x64' > said 2>&1 || { cat said; exit 1; }
        if grep -i 'error\|unknown\|not found\|invalid' said; then exit 1; fi
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

    # swayimg shows a picture with the account's settings, on a screen of
    # its own, and quits once it's drawn. A setting it can't take stops the
    # rest of its settings from being read, said only as it starts, so
    # anything it says fails this. It has the machine's fonts, since with
    # none at all it would say it found no face. Pictures open in swayimg.
    swayimg-accepts-its-configuration =
      let
        home = account graphical;
        files = home.xdg.configFile;
        opens = graphical.config.xdg.mime.defaultApplications."image/png" or "";
        fonts = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };
      in
      assert opens == "swayimg.desktop";
      pkgs.runCommandLocal "swayimg-accepts-its-configuration"
        {
          nativeBuildInputs = [
            home.programs.swayimg.package
            pkgs.cage
          ];
        }
        ''
          export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_RUNTIME_DIR=$PWD/run
          export FONTCONFIG_FILE=${fonts} WLR_BACKENDS=headless WLR_RENDERER=pixman
          export WLR_LIBINPUT_NO_DEVICES=1
          mkdir -p config/swayimg run
          chmod 700 run
          cp ${files."swayimg/init.lua".source} config/swayimg/init.lua
          cp ${files."swayimg/nix.lua".source} config/swayimg/nix.lua
          printf 'P3\n1 1\n255\n0 0 0\n' > picture.ppm
          timeout 30 cage -- swayimg \
            --execute='swayimg.on_redrawn(function() swayimg.exit(0) end)' picture.ppm \
            > said 2>&1 || { cat said; exit 1; }
          # cage's own lines begin with the time.
          if grep -v '^[0-9][0-9]:' said | grep -i -E 'error|warning|deprecated|unable'; then
            exit 1
          fi
          touch $out
        '';

    # zathura opens a page with the account's settings. It names a setting
    # or a colour it can't take and goes on without it, and stays open until
    # it's closed, so it's given a few seconds to say so. Documents open in
    # zathura.
    zathura-accepts-its-configuration =
      let
        home = account graphical;
        opens = graphical.config.xdg.mime.defaultApplications."application/pdf" or "";
        page = import ./tests/page.nix { inherit lib pkgs; };
      in
      assert opens == "org.pwmt.zathura-pdf-mupdf.desktop";
      pkgs.runCommandLocal "zathura-accepts-its-configuration"
        {
          nativeBuildInputs = [
            home.programs.zathura.package
            pkgs.cage
          ];
        }
        ''
          export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
          export XDG_RUNTIME_DIR=$PWD/run GDK_BACKEND=wayland
          export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
          export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
          mkdir -p config/zathura data run
          chmod 700 run
          cp ${home.xdg.configFile."zathura/zathurarc".source} config/zathura/zathurarc
          # cage ends when zathura does, so only a zathura still open at the
          # end of the wait opened.
          status=0
          timeout 5 cage -- zathura ${page} > said 2>&1 || status=$?
          if [ $status -ne 124 ]; then cat said; exit 1; fi
          if grep -E '^(warning|error):' said; then exit 1; fi
          touch $out
        '';

    # Programs change their own git settings with git config --global, which
    # fails on a file the store holds, and what's declared still has to win
    # over what they wrote. So the check runs the switch's own step, with the
    # account's home moved here, then writes as programs do, an include of
    # their own file among them, and switches again. A program that includes
    # its file itself has to find it ahead of the declared settings after the
    # first switch, once, or it adds another below them, as dev does. Neither
    # global file may be a link, or an older generation's link would be in
    # the way of a rollback.
    git-settings-stay-writable =
      let
        home = account (bareNixos [
          self.nixosModules.workstation
          {
            home-manager.users.someone =
              { config, ... }:
              {
                jitsusama.git.programIncludes = [ "${config.xdg.configHome}/program/gitconfig" ];
              };
          }
        ]);
        homeDir = home.home.homeDirectory;
        declared = home.programs.git.settings.user.name;
      in
      pkgs.runCommandLocal "git-settings-stay-writable" { nativeBuildInputs = [ pkgs.git ]; } ''
        export HOME=$PWD/home GIT_CONFIG_NOSYSTEM=1
        mkdir -p home/.config/git
        for linked in .gitconfig .config/git/config; do
          if [ -e ${home.home-files}/$linked ]; then
            echo "~/$linked is a link to the store"
            exit 1
          fi
        done
        cp ${home.home-files}/.config/git/declared.config home/.config/git/
        sed -n '\|declared-last|{s|^run ||;s|${lib.escapeRegex homeDir}|'"$HOME"'|g;p}' \
          ${home.home.activationPackage}/activate > step
        expect() {
          if [ "$(git config --get "$1")" != "$2" ]; then
            echo "$3: $1 is '$(git config --get "$1")', not '$2'"
            exit 1
          fi
        }
        mkdir -p home/.config/program
        printf '[user]\n\tname = program\n[program]\n\tsetting = 1\n' > home/.config/program/gitconfig
        includes() {
          git config --global --no-includes --get-all include.path | tr '\n' ' '
        }
        # The count the program makes before it adds its include, as dev does.
        programs() {
          git config --global --no-includes --get-all include.path |
            sed "s|^~/|$HOME/|" | grep -cxF "$HOME/.config/program/gitconfig"
        }
        sh -e step
        if [ "$(includes)" != "$HOME/.config/program/gitconfig $HOME/.config/git/declared.config " ]; then
          echo "after a switch: ~/.gitconfig includes $(includes)"
          exit 1
        fi
        expect user.name ${lib.escapeShellArg declared} "after a switch"
        expect program.setting 1 "after a switch"
        git config --global --fixed-value --unset-all include.path "$HOME/.config/program/gitconfig"
        git config --global --add include.path "~/.config/program/gitconfig"
        sh -e step
        if [ "$(programs)" != 1 ]; then
          echo "after a switch: the program's include is in ~/.gitconfig $(programs) times"
          exit 1
        fi
        printf '[user]\n\tname = included\n' > home/tool.config
        git config --global --add include.path "$HOME/tool.config"
        git config --global tool.setting 1
        git config --global user.name tool
        expect tool.setting 1 "after a program wrote it"
        sh -e step
        expect user.name ${lib.escapeShellArg declared} "after the next switch"
        expect tool.setting 1 "after the next switch"
        cp home/.gitconfig before
        sh -e step
        if ! cmp -s before home/.gitconfig; then
          echo "a switch with nothing to do changed ~/.gitconfig"
          exit 1
        fi
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

    # The tuple command completes in zsh, which only shows when Tab offers
    # nothing, so the check completes each kind of argument in a real line
    # editor (tests/tuple-completion.zsh). Its tuple is a stand-in, since
    # the real one lists contacts only once signed in, so the real one's
    # settings are held to the shape the completion reads them in.
    tuple-completes-in-zsh =
      let
        home = account graphical;
        tuple = lib.findFirst (package: lib.getName package == "tuple") null home.home.packages;
        completion = lib.findFirst (
          package: lib.getName package == "tuple-zsh-completion"
        ) null home.home.packages;
      in
      pkgs.runCommandLocal "tuple-completes-in-zsh"
        {
          nativeBuildInputs = [
            tuple
            pkgs.zsh
          ];
        }
        ''
          export HOME=$PWD XDG_DATA_HOME=$PWD/data XDG_RUNTIME_DIR=$PWD/run
          mkdir -m 700 run
          tuple settings > settings 2>&1 || true
          tuple off > /dev/null 2>&1 || true
          if ! grep -q '^capture | ' settings || grep -v -E '^[a-z-]+ \| [^|]* \| ' settings; then
            echo "tuple settings no longer lists name | value | what it is:"
            cat settings
            exit 1
          fi
          zsh -f ${./tests/tuple-completion.zsh} \
            ${completion}/share/zsh/site-functions/_tuple \
            ${lib.getExe' pkgs.wl-clipboard "wl-paste"}
          touch $out
        '';

    # A call link opened in a browser comes back as tuple://join, which only
    # the desktop entry's command takes, so the check runs it as the entry
    # does, with stand-ins for tuple and notify-send, over links like the
    # page's and ones it mustn't run. xdg-open finds the entry by what it
    # says it opens, beside a list of defaults a browser wrote, which names
    # none for Tuple's links.
    tuple-opens-its-links =
      let
        home = account graphical;
        tuple = lib.findFirst (package: lib.getName package == "tuple") null home.home.packages;
        entry = home.home.file."${home.xdg.dataHome}/applications/tuple.desktop".text;
      in
      pkgs.runCommandLocal "tuple-opens-its-links" { nativeBuildInputs = [ pkgs.xdg-utils ]; } ''
        mkdir -p data/applications config
        printf '%s' ${lib.escapeShellArg entry} > tuple.desktop
        cp tuple.desktop data/applications/
        printf '[Default Applications]\nx-scheme-handler/https=browser.desktop\n' \
          > config/mimeapps.list
        found=$(HOME=$PWD XDG_DATA_HOME=$PWD/data XDG_DATA_DIRS=$PWD/none \
          XDG_CONFIG_HOME=$PWD/config XDG_CURRENT_DESKTOP=niri \
          xdg-mime query default x-scheme-handler/tuple)
        if [ "$found" != tuple.desktop ]; then
          echo "xdg-open opens Tuple's links with '$found'"
          exit 1
        fi
        opens=$(sed -n 's|^Exec=\(.*\) %u$|\1|p' tuple.desktop)
        sed -e 's|${lib.getExe tuple}|'"$PWD"'/tuple|g' \
          -e 's|/nix/store/[^ ]*/bin/notify-send|'"$PWD"'/notify-send|g' "$opens" > opens
        for stand_in in tuple notify-send; do
          printf '#!/bin/sh\necho "%s $*" > "%s/asked"\n' "$stand_in" "$PWD" > "$stand_in"
        done
        chmod +x opens tuple notify-send

        failed=0
        opening() {
          rm -f asked
          ./opens "$1"
          if [ "$(cat asked)" != "$2" ]; then
            echo "'$1' asked '$(cat asked)', not '$2'"
            failed=1
          fi
        }
        opening 'tuple://join?call_url_slug=U7BVv2' 'tuple join https://tuple.app/c/U7BVv2'
        opening 'tuple://join?from=web&call_url_slug=a_b-9' 'tuple join https://tuple.app/c/a_b-9'
        opening 'tuple://join?call_url_slug=a;b' "notify-send --app-name=Tuple Tuple can't open that link tuple://join?call_url_slug=a;b"
        opening 'tuple://join?call_url_slug=*' "notify-send --app-name=Tuple Tuple can't open that link tuple://join?call_url_slug=*"
        opening 'tuple://auth?code=abc' "notify-send --app-name=Tuple Tuple can't open that link tuple://auth?code=abc"
        opening "" 'tuple on'
        [ "$failed" = 0 ]
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

    # Neovim starts with the configuration an account gets, in the theme's own
    # colour scheme drawn on the terminal's own background, with Tree-sitter
    # drawing a file in a language it has a parser for and its text objects
    # mapped, and nothing it loads complains.
    neovim-starts-in-the-theme =
      let
        home = account workstation;
        report = lib.concatStringsSep " .. ' ' .. " [
          "vim.g.colors_name"
          "tostring(vim.api.nvim_get_hl(0, { name = 'Normal' }).bg)"
          "tostring(vim.treesitter.highlighter.active[vim.api.nvim_get_current_buf()] ~= nil)"
          "(vim.fn.mapcheck('af', 'o') ~= '' and 'objects' or 'none')"
        ];
      in
      pkgs.runCommandLocal "neovim-starts-in-the-theme"
        {
          # The programs the account gets, git and gh among them, which
          # plugins call as they start.
          nativeBuildInputs = [ home.home.path ];
        }
        ''
          ${neovimAt home}
          echo 'return {}' > probe.lua
          nvim --headless probe.lua -c "lua io.stdout:write(${report})" -c 'qa!' > answer 2> complaints
          if [ -s complaints ]; then cat complaints; exit 1; fi
          if [ "$(cat answer)" != "jitsusama nil true objects" ]; then cat answer; exit 1; fi
          touch $out
        '';

    # Neovim starts with only what draws its first screen: with a file open,
    # no plugin that waits has loaded, nor vim.diagnostic, yet every command
    # a key runs is there, standing in. A command run, a module required and
    # an action asked for each load their plugin, the background ones load
    # once the first screen is drawn, and the Lua language server reads
    # only Neovim's own runtime.
    neovim-starts-with-only-what-it-draws =
      let
        home = account workstation;
        probe = pkgs.writeText "probe.lua" ''
          local later = require('later')
          local keys = require('keys')
          local wrong = {}
          local function say(line)
            table.insert(wrong, line)
          end
          for name in pairs(require('waiting')) do
            if later.loaded(name) then
              say(name .. ' loaded as Neovim started')
            end
          end
          -- What draws the first screen, and what those need.
          local first = {
            ['async.nvim'] = true,
            ['nvim-numbertoggle'] = true,
            ['nvim-treesitter'] = true,
            ['nvim-treesitter-grammars'] = true,
            ['nvim-treesitter-textobjects'] = true,
            ['oil.nvim'] = true,
            ['snacks.nvim'] = true,
            ['vim-nix'] = true,
            ['vim-repeat'] = true,
          }
          for _, dir in ipairs(vim.fn.globpath(vim.o.packpath, 'pack/*/start/*', false, true)) do
            if not first[vim.fs.basename(dir)] then
              say(vim.fs.basename(dir) .. ' starts with Neovim, though the first screen needs none of it')
            end
          end
          if package.loaded['vim.diagnostic'] then
            say('vim.diagnostic loaded as Neovim started')
          end
          for _, key in ipairs(keys.keys) do
            local command = key.command and key.command:match('^%S+')
            if command and vim.fn.exists(':' .. command) ~= 2 then
              say(key.lhs .. ' runs ' .. command .. ', which is not there')
            end
          end
          local windows = #vim.api.nvim_list_wins()
          vim.cmd('UndotreeShow')
          if not later.loaded('undotree') or #vim.api.nvim_list_wins() == windows then
            say('UndotreeShow did not load undotree and show it')
          end
          require('trouble')
          if not later.loaded('trouble.nvim') then
            say('requiring trouble did not load it')
          end
          -- aerial's setup requires aerial itself, so the first require has to
          -- answer with the copy that was set up; another copy has no settings
          -- and fails when asked for an outline. This file has no symbols, so
          -- the outline it gives is empty.
          local opened, problem = pcall(function()
            require('aerial').nav_open()
          end)
          if not opened then
            say('aerial, called before it loaded, was not set up: ' .. tostring(problem))
          end
          later.provide('jump.line')
          if not later.loaded('flash.nvim') or keys.actions['jump.line'] == nil then
            say('asking for jump.line did not load flash')
          end
          vim.api.nvim_exec_autocmds('UIEnter', {})
          local background = { 'nvim-lspconfig', 'nvim-cmp', 'gitsigns.nvim', 'which-key.nvim' }
          vim.wait(5000, function()
            return vim.iter(background):all(later.loaded)
          end)
          for _, name in ipairs(background) do
            if not later.loaded(name) then
              say(name .. ' did not load once the first screen was drawn')
            end
          end
          -- Completion's sources register in their after/plugin files.
          vim.wait(2000, function()
            return #require('cmp').core:get_sources() >= 2
          end)
          local sources = {}
          for _, source in ipairs(require('cmp').core:get_sources()) do
            sources[source.name] = true
          end
          if not (sources.buffer and sources.path) then
            say('suggestions come from ' .. table.concat(vim.tbl_keys(sources), ' '))
          end
          local library = vim.lsp.config.lua_ls.settings.Lua.workspace.library
          if not vim.deep_equal(library, { vim.env.VIMRUNTIME, "''${3rd}/luv/library" }) then
            say('lua_ls reads ' .. #library .. ' folders')
          end
          io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
          vim.cmd('qa!')
        '';
      in
      pkgs.runCommandLocal "neovim-starts-with-only-what-it-draws"
        { nativeBuildInputs = [ home.home.path ]; }
        ''
          ${neovimAt home}
          echo 'return {}' > probe.lua
          # From VimEnter, which a person meets Neovim after, rather than from
          # a -c command, which runs before it. A probe that fails says why and
          # quits, and one that never finishes is stopped, so the check fails
          # rather than waits.
          timeout 120 nvim --headless --cmd 'autocmd VimEnter * ++once lua local ok, problem = xpcall(dofile, debug.traceback, "${probe}") if not ok then io.stdout:write(problem) vim.cmd("qa!") end' probe.lua \
            > answer 2> complaints || true
          if [ -s complaints ]; then cat complaints; exit 1; fi
          if [ ! -s answer ]; then echo 'Neovim did not finish in two minutes'; exit 1; fi
          if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
          touch $out
        '';

    # Neovim's floats stand apart by their shade, with a step of it round
    # their words rather than a line, and nothing behind them shows through:
    # a problem shown in full, a language server's note, a notice, the
    # suggestions, the outline and a change's preview. A notice's mark comes
    # before its words.
    neovim-floats-by-shade =
      let
        home = account workstation;
        probe = pkgs.writeText "probe.lua" ''
          require('later').load_all()
          local wrong = {}
          local function say(line)
            table.insert(wrong, line)
          end
          local function solid(where, border)
            if border ~= 'solid' then
              say(where .. ' has the border ' .. vim.inspect(border))
            end
          end
          -- A float Neovim drew: its border all spaces, and nothing through it.
          local function look(where, win)
            if not win or not vim.api.nvim_win_is_valid(win) then
              return say(where .. ' opened no float')
            end
            local border = vim.api.nvim_win_get_config(win).border
            local spaces = type(border) == 'table' and #border == 8
            for _, part in ipairs(type(border) == 'table' and border or {}) do
              if (type(part) == 'table' and part[1] or part) ~= ' ' then
                spaces = false
              end
            end
            if not spaces then
              say(where .. ' is drawn with the border ' .. vim.inspect(border))
            end
            if vim.wo[win].winblend ~= 0 then
              say(where .. ' shows what is behind it')
            end
          end

          if vim.o.pumblend ~= 0 then
            say('the suggestions show what is behind them')
          end
          local namespace = vim.api.nvim_create_namespace('probe')
          vim.diagnostic.set(namespace, 0, {
            { lnum = 0, col = 0, message = 'a problem', severity = vim.diagnostic.severity.WARN },
          })
          local _, problem = vim.diagnostic.open_float()
          look('a problem shown in full', problem)
          pcall(vim.api.nvim_win_close, problem, true)
          local _, note = vim.lsp.util.open_floating_preview({ 'a note' }, 'markdown', {})
          look("a language server's note", note)
          pcall(vim.api.nvim_win_close, note, true)

          vim.notify('a notice', vim.log.levels.INFO)
          local notice
          vim.wait(2000, function()
            for _, win in ipairs(vim.api.nvim_list_wins()) do
              if vim.bo[vim.api.nvim_win_get_buf(win)].filetype == 'snacks_notif' then
                notice = win
              end
            end
            return notice ~= nil
          end)
          look('a notice', notice)
          if notice then
            local first = vim.api.nvim_buf_get_lines(vim.api.nvim_win_get_buf(notice), 0, 1, false)[1]
            local mark = require('theme').glyphs.note
            if first:sub(1, #mark) ~= mark then
              say('a notice reads "' .. first .. '", not its mark and then its words')
            end
          end

          local window = require('cmp').get_config().window
          solid('the suggestions', window.completion.border)
          solid("a suggestion's notes", window.documentation.border)
          -- aerial puts its settings in place when first used.
          require('aerial').sync_load()
          local aerial = require('aerial.config')
          solid('the outline', aerial.nav.border)
          solid('the outline beside the cursor', aerial.float.border)
          if aerial.nav.win_opts.winblend ~= 0 then
            say('the outline shows what is behind it')
          end
          solid("a change's preview", require('gitsigns.config').config.preview_config.border)

          -- The finder: on the floats' shade, nothing dimmed behind it, and
          -- its title at the list's edge rather than a space in.
          local finder = require('fzf-lua.config').globals.winopts
          solid('the finder', finder.border)
          solid("the finder's preview", finder.preview.border)
          if finder.backdrop ~= false then
            say('the finder dims what is behind it')
          end
          local shade = vim.api.nvim_get_hl(0, { name = 'NormalFloat', link = false }).bg
          if vim.api.nvim_get_hl(0, { name = 'FzfLuaNormal', link = false }).bg ~= shade then
            say("the finder isn't on the floats' shade")
          end
          local titled = require('fzf-lua.utils').nvim_open_win(vim.api.nvim_create_buf(false, true), false, {
            relative = 'editor', row = 1, col = 1, width = 20, height = 2, title = { { ' Files ', 'FloatTitle' } },
          })
          local title = vim.api.nvim_win_get_config(titled).title
          if title[1][1] ~= 'Files' then
            say('the finder is titled "' .. title[1][1] .. '", not from its edge')
          end

          -- While a float has the keys, the status line names the file under it.
          vim.api.nvim_set_current_win(titled)
          local named = {}
          for _, piece in ipairs((require('status').layout(200))) do
            table.insert(named, piece[2])
          end
          if not table.concat(named):find('probe.lua', 1, true) then
            say('the status line names "' .. table.concat(named) .. '" over a float, not the file under it')
          end
          io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
          vim.cmd('qa!')
        '';
      in
      pkgs.runCommandLocal "neovim-floats-by-shade" { nativeBuildInputs = [ home.home.path ]; } ''
        ${neovimAt home}
        echo 'return {}' > probe.lua
        timeout 120 nvim --headless --cmd 'autocmd VimEnter * ++once lua local ok, problem = xpcall(dofile, debug.traceback, "${probe}") if not ok then io.stdout:write(problem) vim.cmd("qa!") end' probe.lua \
          > answer 2> complaints || true
        if [ -s complaints ]; then cat complaints; exit 1; fi
        if [ ! -s answer ]; then echo 'Neovim did not finish in two minutes'; exit 1; fi
        if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
        touch $out
      '';

    # Space in any of Neovim's groups finds among what it holds: every group
    # keys.nix gives keys has its own Space, and one whose keys are things
    # to do lists exactly its keys that work here, each by what it does in
    # strong and then its keys in text, in a column of their own, titled by
    # the group, and runs the one chosen as if pressed. A group with nothing
    # that works here says so rather than opening an empty list.
    neovim-finds-in-every-group =
      let
        home = account workstation;
        chords = map (key: key.chord) (import ./modules/neovim/keys.nix);
        groups = lib.unique (
          lib.concatMap (
            chord:
            let
              words = lib.splitString " " chord;
            in
            lib.optionals (lib.head words == "Space" && lib.length words > 2) (
              map (length: lib.concatStringsSep " " (lib.take length words)) (lib.range 2 (lib.length words - 1))
            )
          ) chords
        );
        lacking = lib.filter (group: !(lib.elem "${group} Space" chords)) groups;
        probe = pkgs.writeText "probe.lua" ''
          -- Every plugin loaded first, so none replaces the stand-ins after.
          require('later').load_all()
          local keys = require('keys')
          local wrong = {}
          local function say(line)
            table.insert(wrong, line)
          end
          local offered, rows, chosen, told, titled
          vim.ui.select = function(items, opts, on_choice)
            offered, rows, titled = {}, #items, opts.prompt
            for _, item in ipairs(items) do
              offered[item.rest] = opts.format_item(item)
            end
            chosen = function(rest)
              for _, item in ipairs(items) do
                if item.rest == rest then
                  return on_choice(item)
                end
              end
            end
          end
          vim.notify = function(message)
            told = message
          end

          local looks = 0
          for _, key in ipairs(keys.keys) do
            if key.chord:match('^Space u %S+$') and key.chord ~= 'Space u Space' then
              looks = looks + 1
            end
          end
          keys.find('Space u', 'looks')
          local count = vim.tbl_count(offered or {})
          if count ~= looks then
            say('Space u Space offers ' .. count .. ' keys, not its ' .. looks)
          end
          local function paint(role, words)
            local hex = require('theme')[role]
            return ('\27[38;2;%d;%d;%dm%s\27[0m'):format(
              tonumber(hex:sub(2, 3), 16), tonumber(hex:sub(4, 5), 16), tonumber(hex:sub(6, 7), 16), words)
          end
          local row = (offered or {}).w or '''
          local said = row:match('^' .. vim.pesc(paint('strong', 'Wrap long lines, or not')) .. ' +')
          if not said or row ~= said .. paint('text', 'w') then
            say('Space u Space shows w as "' .. vim.inspect(row) .. '"')
          end
          local columns = {}
          for rest, line in pairs(offered or {}) do
            local plain = line:gsub('\27%[[%d;]*m', ''')
            columns[#plain - #rest] = true
          end
          if vim.tbl_count(columns) ~= 1 then
            say('Space u Space puts its keys in ' .. vim.tbl_count(columns) .. ' columns, not one')
          end
          if titled ~= 'Looks' then
            say('Space u Space is titled "' .. tostring(titled) .. '"')
          end
          local wrapped = vim.wo.wrap
          chosen('w')
          if vim.wo.wrap == wrapped then
            say('choosing w from Space u Space left wrapping as it was')
          end

          -- In an issue, its comments' keys, once each, though octo's other
          -- windows give the same chords keys of their own.
          vim.cmd('enew')
          require('octo.utils').apply_mappings('issue', 0)
          offered = nil
          keys.find('Space g h c', 'comments')
          local comments = vim.tbl_keys(offered or {})
          table.sort(comments)
          if table.concat(comments, ' ') ~= 'Return h i r w' or rows ~= 5 then
            say('Space g h c Space in an issue offers ' .. tostring(rows) .. ': ' .. table.concat(comments, ', '))
          end
          -- And in a review's change, its own comment key, not an issue's.
          vim.cmd('enew')
          require('octo.utils').apply_mappings('review_diff', 0)
          offered = nil
          keys.find('Space g h c', 'comments')
          local comment = (offered or {}).Return or '''
          if not comment:find('Comment on the line', 1, true) then
            say('Space g h c Space in a change offers Return as "' .. comment .. '"')
          end
          vim.cmd('bwipeout!')
          vim.cmd('bwipeout!')

          offered = nil
          keys.find('Space g s', 'stage')
          if offered ~= nil then
            say('Space g s Space offers git keys in a file git does not track')
          elseif told ~= 'No key for stage works here' then
            say('Space g s Space with nothing to offer said "' .. tostring(told) .. '"')
          end
          io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
          vim.cmd('qa!')
        '';
      in
      if lacking != [ ] then
        holds "neovim-finds-in-every-group"
          "These groups have no Space: ${lib.concatStringsSep ", " lacking}."
          false
      else
        pkgs.runCommandLocal "neovim-finds-in-every-group" { nativeBuildInputs = [ home.home.path ]; } ''
          ${neovimAt home}
          echo 'return {}' > probe.lua
          timeout 120 nvim --headless --cmd 'autocmd VimEnter * ++once lua local ok, problem = xpcall(dofile, debug.traceback, "${probe}") if not ok then io.stdout:write(problem) vim.cmd("qa!") end' probe.lua \
            > answer 2> complaints || true
          if [ -s complaints ]; then cat complaints; exit 1; fi
          if [ ! -s answer ]; then echo 'Neovim did not finish in two minutes'; exit 1; fi
          if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
          touch $out
        '';

    # Neovim's status line, drawn for a file in a repository with changes
    # not yet committed, with git's state from the engine: the file told
    # from its root, the branch with its unsaved mark, a mark once the
    # buffer changes, an error counted, the mode once it isn't Normal, and
    # all of it fitting a narrow window.
    neovim-draws-its-status-line =
      let
        home = account workstation;
        probe = pkgs.writeText "probe.lua" ''
          local status = require('status')
          local g = require('theme').glyphs
          local wrong = {}
          local function plain(pieces)
            local text = {}
            for _, piece in ipairs(pieces) do
              table.insert(text, piece[2])
            end
            return table.concat(text)
          end
          local function expect(what, columns, left, right)
            local l, r = status.layout(columns)
            l, r = plain(l), plain(r)
            if l ~= left or r ~= right then
              table.insert(wrong, what .. ': "' .. l .. '" and "' .. r .. '"')
            end
          end
          vim.wait(5000, function()
            return #select(2, status.layout(200)) > 0
          end)
          expect('at rest', 120, 'project/sub/notes.txt', g.branch .. ' main ' .. g.on)
          vim.api.nvim_buf_set_lines(0, -1, -1, false, { 'more' })
          expect('changed', 120, 'project/sub/notes.txt ' .. g.on, g.branch .. ' main ' .. g.on)
          vim.diagnostic.set(vim.api.nvim_create_namespace('probe'), 0, {
            { lnum = 0, col = 0, severity = vim.diagnostic.severity.ERROR, message = 'e' },
          })
          expect('an error', 120, 'project/sub/notes.txt ' .. g.on,
            g.failed .. ' 1  ' .. g.branch .. ' main ' .. g.on)
          expect('narrow', 35, 'project/' .. g.more .. '/notes.txt ' .. g.on,
            g.failed .. ' 1  ' .. g.branch .. ' ' .. g.on .. '  ' .. g.more)
          vim.api.nvim_feedkeys('i', 'n', false)
          vim.defer_fn(function()
            expect('typing', 120, 'project/sub/notes.txt ' .. g.on .. '  insert',
              g.failed .. ' 1  ' .. g.branch .. ' main ' .. g.on)
            vim.cmd('stopinsert')
            vim.defer_fn(function()
              -- The file as fugitive opens it in the index: told from its
              -- root as the file is, the version after it, git's state its
              -- folder's.
              vim.cmd('enew')
              vim.bo.buftype = 'nowrite'
              vim.api.nvim_buf_set_name(0, 'fugitive://' .. vim.fn.getcwd() .. '/project/.git//0/sub/notes.txt')
              expect('a version', 120, 'project/sub/notes.txt  index', g.branch .. ' main ' .. g.on)
              -- A terminal by its program, not its whole term:// address.
              vim.cmd('terminal sleep 5')
              expect('a terminal', 120, 'terminal sleep 5', ''')
              io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
              vim.cmd('qa!')
            end, 100)
          end, 200)
        '';
      in
      pkgs.runCommandLocal "neovim-draws-its-status-line" { nativeBuildInputs = [ home.home.path ]; } ''
        ${neovimAt home}
        mkdir -p project/sub
        # Text, which no language server reads, so none reports its progress.
        echo 'a first line' > project/sub/notes.txt
        git -C project init -q -b main
        git -C project add -A
        git -C project -c user.name=check -c user.email=check@localhost -c commit.gpgsign=false \
          commit -q -m first
        echo 'a second line' >> project/sub/notes.txt
        nvim --headless project/sub/notes.txt -c 'luafile ${probe}' > answer 2> complaints
        if [ -s complaints ]; then cat complaints; exit 1; fi
        if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
        touch $out
      '';

    # Neovim's numbers stand in one column: the line the cursor is on says
    # its own number, marked, right-aligned with the counts above and below
    # it rather than pushed left, and a wrapped line's later rows say none.
    neovim-keeps-its-numbers-in-one-column =
      let
        home = account workstation;
        probe = pkgs.writeText "probe.lua" ''
          local wrong = {}
          vim.wo.wrap = true
          vim.api.nvim_win_set_cursor(0, { 61, 0 })
          local function column(lnum)
            return vim.api.nvim_eval_statusline(vim.o.statuscolumn, {
              use_statuscol_lnum = lnum, highlights = true, winid = 0,
            })
          end
          local above, here, below = column(60), column(61), column(62)
          if not here.str:match('^%s*61 $') then
            table.insert(wrong, 'the line here reads "' .. here.str .. '"')
          end
          if not above.str:match('^%s*1 $') or not below.str:match('^%s*1 $') then
            table.insert(wrong, 'the lines beside it read "' .. above.str .. '" and "' .. below.str .. '"')
          end
          if #above.str ~= #here.str or #below.str ~= #here.str then
            table.insert(wrong, 'the numbers are not one width: "' .. above.str .. '", "' .. here.str .. '", "' .. below.str .. '"')
          end
          local marked = false
          for _, mark in ipairs(here.highlights) do
            marked = marked or mark.group == 'CursorLineNr'
          end
          if not marked then
            table.insert(wrong, 'the line here is not marked as the cursor line')
          end
          vim.wo.relativenumber, vim.wo.number = false, false
          if vim.trim(column(60).str) ~= ''' then
            table.insert(wrong, 'with numbers off the column says "' .. column(60).str .. '"')
          end
          io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
          vim.cmd('qa!')
        '';
      in
      pkgs.runCommandLocal "neovim-keeps-its-numbers-in-one-column"
        { nativeBuildInputs = [ home.home.path ]; }
        ''
          ${neovimAt home}
          seq 1 120 > lines.txt
          nvim --headless lines.txt -c 'luafile ${probe}' > answer 2> complaints
          if [ -s complaints ]; then cat complaints; exit 1; fi
          if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
          touch $out
        '';

    # Neovim's spell check in a markdown file knows the names this writing
    # uses, and still finds a word spelt wrong.
    neovim-knows-the-names-it-writes =
      let
        home = account workstation;
        probe = pkgs.writeText "probe.lua" ''
          local line = vim.api.nvim_buf_get_lines(0, 0, 1, false)[1]
          local flagged = {}
          for _, bad in ipairs(vim.spell.check(line)) do
            table.insert(flagged, bad[1])
          end
          local said = table.concat(flagged, ' ')
          io.stdout:write(vim.wo.spell and said == 'speling' and 'as designed'
            or ('spell ' .. tostring(vim.wo.spell) .. ', flagged: ' .. said))
          vim.cmd('qa!')
        '';
      in
      pkgs.runCommandLocal "neovim-knows-the-names-it-writes" { nativeBuildInputs = [ home.home.path ]; }
        ''
          ${neovimAt home}
          echo "Neovim's niri and Quickshell, a YubiKey and the TPM, in monospace: a speling slip." > notes.md
          nvim --headless notes.md -c 'luafile ${probe}' > answer 2> complaints
          if [ -s complaints ]; then cat complaints; exit 1; fi
          if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
          touch $out
        '';

    # The account's Neovim binds every key in the map, and only those: each
    # action a key names is one a file registered, or one a plugin that
    # waits provides, each key is bound as it says, a scope's in a buffer it
    # arrives in, each plugin's window holds its keys and none of the
    # plugin's own, and no key Neovim has that says what it does is missing
    # from the map. Every plugin is loaded first, as it would be once used.
    neovim-binds-every-key =
      let
        home = account workstation;
        probe = pkgs.writeText "probe.lua" ''
          local keys = require('keys')
          local later = require('later')
          local wrong = {}
          local function say(line)
            table.insert(wrong, line)
          end
          for _, name in ipairs(keys.missing()) do
            if not later.provides(name) then
              say('nothing provides ' .. name)
            end
          end
          later.load_all()
          for _, name in ipairs(keys.missing()) do
            say('no action ' .. name)
          end
          -- Every scope arrives in this one buffer, as each would in its own.
          for _, scope in ipairs({ 'code', 'git', 'status' }) do
            keys.attach(scope, 0)
          end
          local ours = {}
          for _, key in ipairs(vim.tbl_filter(function(key)
            return key.plugin == nil
          end, keys.keys)) do
            for _, mode in ipairs(key.modes) do
              local map = vim.fn.maparg(key.lhs, mode, false, true)
              if map.desc ~= key.desc then
                say(mode .. ' ' .. key.lhs .. ' is ' .. tostring(map.desc))
              elseif (map.buffer == 1) ~= (key.scope ~= nil) then
                say(mode .. ' ' .. key.lhs .. ' is in the wrong place')
              else
                ours[mode .. map.lhsraw] = true
              end
            end
          end
          for _, mode in ipairs({ 'n', 'x', 'o', 'i', 't' }) do
            local maps = vim.list_extend(
              vim.api.nvim_get_keymap(mode),
              vim.api.nvim_buf_get_keymap(0, mode)
            )
            for _, map in ipairs(maps) do
              local raw = vim.fn.maparg(map.lhs, mode, false, true).lhsraw
              -- A key set once Neovim had started has no script, nor was it
              -- set by the configuration.
              local from = map.sid > 0
                  and (vim.fn.getscriptinfo({ sid = map.sid })[1] or {}).name
                or ""
              if map.desc and not ours[mode .. raw] and from:find('/config/nvim/', 1, true) then
                say('stray ' .. mode .. ' ' .. map.lhs .. ' ' .. map.desc)
              end
            end
          end

          -- A plugin's window binds its own keys, so each window is opened
          -- as it would be, and holds its keys from keys.nix and no others
          -- but a mouse's.
          vim.fn.writefile({ 'local function hello()', 'end' }, 'probe.lua')
          vim.cmd('edit probe.lua')
          local function window(scope, filetype, open, theirs)
            open()
            if not vim.wait(5000, function() return vim.bo.filetype == filetype end) then
              say('no ' .. scope .. ' window')
              return
            end
            local mine, modes = {}, {}
            for _, key in ipairs(keys.keys) do
              if key.scope == scope then
                for _, mode in ipairs(key.modes) do
                  modes[mode] = true
                  local map = vim.fn.maparg(key.lhs, mode, false, true)
                  if map.desc ~= key.desc or map.buffer ~= 1 then
                    say(scope .. ' ' .. mode .. ' ' .. key.lhs .. ' is ' .. tostring(map.desc))
                  else
                    mine[mode .. map.lhsraw] = true
                  end
                end
              end
            end
            for mode in pairs(modes) do
              for _, map in ipairs(vim.api.nvim_buf_get_keymap(0, mode)) do
                local raw = vim.fn.maparg(map.lhs, mode, false, true).lhsraw
                if not mine[mode .. raw] and not map.lhs:find('Mouse') and not (theirs and theirs(map)) then
                  say('stray ' .. scope .. ' ' .. mode .. ' ' .. map.lhs .. ' ' .. tostring(map.desc))
                end
              end
            end
          end
          window('outline', 'aerial', function()
            require('aerial').open({ focus = true })
          end)
          vim.cmd('AerialClose')
          window('walk', 'aerial-nav', function()
            require('aerial').nav_open()
          end)
          vim.wait(200)
          require('aerial').nav_close()
          vim.wait(200)
          window('files', 'oil', function()
            vim.cmd('Oil .')
          end)
          vim.cmd('edit probe.lua')
          window('problems', 'trouble', function()
            vim.fn.setqflist({ { filename = 'probe.lua', lnum = 1, text = 'a problem' } })
            require('trouble').toggle('quickfix')
          end)
          -- octo binds a buffer's keys as it opens one, through apply_mappings,
          -- and its notifications' in the finder, from its configuration.
          local octo = {
            ['octo-issue'] = 'issue',
            ['octo-pull'] = 'pull_request',
            ['octo-thread'] = 'review_thread',
            ['octo-submit'] = 'submit_win',
            ['octo-change'] = 'review_diff',
            ['octo-files'] = 'file_panel',
          }
          for _, scope in ipairs(vim.tbl_keys(octo)) do
            -- autopairs binds its own keys for typing in every buffer.
            window(scope, 'octo-probe', function()
              vim.cmd('enew')
              vim.bo.filetype = 'octo-probe'
              require('octo.utils').apply_mappings(octo[scope], 0)
            end, function(map)
              return (map.desc or '''):find('^autopairs') ~= nil
            end)
          end
          -- Its notifications' and workflow runs' keys octo binds its own
          -- way, so they're read from its configuration, and its runs'
          -- view, which reads them as it loads, loads.
          local given = require('octo.config').values.mappings
          local theirs = { ['octo-notices'] = 'notification', ['octo-runs'] = 'runs' }
          for _, key in ipairs(keys.keys) do
            if theirs[key.scope] then
              local action = (given[theirs[key.scope]] or {})[key.plugin] or {}
              if action.lhs ~= key.lhs or action.desc ~= key.desc then
                say(key.scope .. ' ' .. key.lhs .. ' is not ' .. key.plugin)
              end
            end
          end
          local loaded, problem = pcall(require, 'octo.workflow_runs')
          if not loaded then
            say('octo cannot show a workflow run: ' .. tostring(problem))
          end
          -- cmp binds the suggestions' keys as it suggests, from these.
          local mapping = require('cmp').get_config().mapping
          local suggesting = vim.tbl_filter(function(key)
            return key.scope == 'completion'
          end, keys.keys)
          for _, key in ipairs(suggesting) do
            if mapping[require('cmp.utils.keymap').normalize(key.lhs)] == nil then
              say('completion ' .. key.lhs .. ' is unbound')
            end
          end
          if vim.tbl_count(mapping) ~= #suggesting then
            say('completion has ' .. vim.tbl_count(mapping) .. ' keys, not ' .. #suggesting)
          end
          -- The command line's are its own, for a command and a search alike.
          local lines = require('cmp.config').cmdline
          local commanding = vim.tbl_filter(function(key)
            return key.scope == 'commands'
          end, keys.keys)
          for _, kind in ipairs({ ':', '/', '?' }) do
            local line = require('cmp.config').normalize(lines[kind] or {}).mapping or {}
            for _, key in ipairs(commanding) do
              local bound = line[require('cmp.utils.keymap').normalize(key.lhs)]
              if bound == nil or bound.c == nil then
                say('command line ' .. kind .. ' ' .. key.lhs .. ' is unbound')
              end
            end
            if vim.tbl_count(line) ~= #commanding then
              say('command line ' .. kind .. ' has ' .. vim.tbl_count(line) .. ' keys, not ' .. #commanding)
            end
          end
          io.stdout:write(#wrong == 0 and 'every key' or table.concat(wrong, '\n'))
        '';
      in
      pkgs.runCommandLocal "neovim-binds-every-key" { nativeBuildInputs = [ home.home.path ]; } ''
        ${neovimAt home}
        # Neovim says where each key was set only when it's verbose, which
        # goes to a file rather than among the complaints.
        nvim --headless --cmd 'set verbose=1 verbosefile=verbose' \
          -c 'set verbose=0' -c 'luafile ${probe}' -c 'qa!' > answer 2> complaints
        if [ -s complaints ]; then cat complaints; exit 1; fi
        if [ "$(cat answer)" != "every key" ]; then cat answer; exit 1; fi
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

    # The ErgoDox's layout compiles into firmware that fits the board, and the
    # board will answer with the build's own serial number, never Oryx's, so
    # ZSA's tools don't take it for an Oryx layout and replace it. USB gives
    # a serial number in UTF-16.
    the-ergodox-layout-builds =
      let
        firmware = import ./modules/ergodox/firmware.nix pkgs {
          firmware = self.inputs.qmk;
          inherit (self.inputs) lufa printf;
        };
      in
      pkgs.runCommandLocal "the-ergodox-layout-builds"
        {
          nativeBuildInputs = [
            pkgs.pkgsCross.avr.buildPackages.binutils
            pkgs.python3
          ];
        }
        ''
          avr-objcopy -I ihex -O binary ${firmware}/ergodox.hex image
          python3 - ${firmware}/serial <<'EOF'
          import sys
          image = open("image", "rb").read()
          serial = open(sys.argv[1]).read().strip()
          if serial.encode("utf-16-le") not in image:
              sys.exit(f"The board wouldn't answer with the build's serial, {serial}.")
          if "omjyr".encode("utf-16-le") in image:
              sys.exit("The board would still answer with Oryx's serial.")
          EOF
          touch $out
        '';

    # The ergodox command finds the board's channel in a copy of sysfs by
    # ZSA's vendor number and Oryx's usage page together, and writes each
    # command as one report. Another vendor's raw channel and the board's
    # keyboard get nothing, and with no channel the command says so.
    the-ergodox-command-reaches-the-board =
      let
        ergodox = lib.findFirst (
          package: lib.getName package == "ergodox"
        ) (throw "No ergodox command.") (account workstation).home.packages;
      in
      pkgs.runCommandLocal "the-ergodox-command-reaches-the-board" { } ''
        node() {
          mkdir -p root/sys/class/hidraw/$1/device root/dev
          echo "HID_ID=$2" > root/sys/class/hidraw/$1/device/uevent
          printf "$3" > root/sys/class/hidraw/$1/device/report_descriptor
          : > root/dev/$1
        }
        node hidraw0 0003:0000046D:0000C52B '\006\140\377\011\141'
        node hidraw1 0003:00003297:00004976 '\005\001\011\006'
        node hidraw2 0003:00003297:00004976 '\006\140\377\011\141'
        export ERGODOX_ROOT=$PWD/root

        sent() { od -An -tx1 -v root/dev/$1 | tr -d ' \n'; }
        expect() {
          ${ergodox}/bin/ergodox $1 $2
          want=00$3$(printf '%060d' 0)
          [ "$(sent hidraw2)" = "$want" ] || {
            echo "ergodox $1 $2 wrote $(sent hidraw2), not $want."
            exit 1
          }
          [ -z "$(sent hidraw0)$(sent hidraw1)" ] || {
            echo "ergodox $1 $2 wrote to a node that isn't the board's channel."
            exit 1
          }
        }
        expect bootloader "" 8000
        expect layer numb 8202
        expect layer Punk 8201
        expect layer 0 8200
        expect lights off 8300
        expect brighter "" 8401
        expect dimmer "" 8400

        status=0
        ${ergodox}/bin/ergodox layer sideways 2> /dev/null || status=$?
        [ $status = 64 ] || { echo "An unknown layer exited $status, not 64."; exit 1; }

        rm -r root/sys/class/hidraw/hidraw2 root/dev/hidraw2
        status=0
        ${ergodox}/bin/ergodox bootloader 2> /dev/null || status=$?
        [ $status = 69 ] || { echo "With no board, bootloader exited $status, not 69."; exit 1; }
        [ -z "$(sent hidraw0)$(sent hidraw1)" ] || {
          echo "With no board, ergodox wrote to another node."
          exit 1
        }
        touch $out
      '';

    # The layer watcher's own tests: it finds the board's channel as the
    # command does, asks to watch in one report, and says only what a watch
    # report tells it, by the layer's name.
    # linux-id's pinentry speaks pinentry's protocol and asks the askpass in
    # the words the program set, as its own tests say.
    the-pinentry-asks-the-shell =
      pkgs.runCommandCC "the-pinentry-asks-the-shell" { nativeBuildInputs = [ pkgs.rustc ]; }
        ''
          rustc --edition 2024 --test -D warnings -o tests ${./modules/linux-id/pinentry.rs}
          ./tests
          touch $out
        '';

    # The click strength's setter finds the intensity in a haptic touchpad's
    # report descriptor, the XPS 14's among them, passes over a device
    # without one, and writes only that field, only when it differs.
    the-click-strength-finds-the-intensity =
      pkgs.runCommandCC "the-click-strength-finds-the-intensity" { nativeBuildInputs = [ pkgs.rustc ]; }
        ''
          rustc --edition 2024 --test -D warnings -o tests ${./modules/touchpad/click-strength.rs}
          ./tests
          touch $out
        '';

    the-ergodox-watch-tells-the-layer =
      pkgs.runCommandCC "the-ergodox-watch-tells-the-layer" { nativeBuildInputs = [ pkgs.rustc ]; }
        ''
          rustc --edition 2024 --test -D warnings -o tests ${./modules/ergodox/ergodox-watch.rs}
          ./tests
          touch $out
        '';

    # systemd-tmpfiles turns Btrfs's reclaim on for every filesystem mounted,
    # data and metadata, and leaves the system chunks alone. It writes into a
    # copy of sysfs with two filesystems, since a pattern that matches none
    # passes without a word.
    btrfs-reclaims-its-chunks =
      let
        rules = lib.filter (lib.hasInfix "/sys/fs/btrfs/") workstation.config.systemd.tmpfiles.rules;
      in
      pkgs.runCommandLocal "btrfs-reclaims-its-chunks" { nativeBuildInputs = [ pkgs.systemd ]; } ''
        for fs in one two; do
          for kind in data metadata system; do
            mkdir -p root/sys/fs/btrfs/$fs/allocation/$kind
            for setting in dynamic_reclaim periodic_reclaim; do
              echo 0 > root/sys/fs/btrfs/$fs/allocation/$kind/$setting
            done
          done
        done
        systemd-tmpfiles --root=$PWD/root --create ${pkgs.writeText "btrfs.conf" (lib.concatLines rules)}
        for fs in one two; do
          for setting in dynamic_reclaim periodic_reclaim; do
            for kind in data metadata; do
              if [ "$(cat root/sys/fs/btrfs/$fs/allocation/$kind/$setting)" != 1 ]; then
                echo "Btrfs's $setting is off for $kind."
                exit 1
              fi
            done
            if [ "$(cat root/sys/fs/btrfs/$fs/allocation/system/$setting)" != 0 ]; then
              echo "Btrfs's $setting was turned on for the system chunks."
              exit 1
            fi
          done
        done
        touch $out
      '';

    # The disk guard reads the configs a workstation and its account get:
    # each is refused at build by the guard's own check-config, which builds
    # the guard and runs its scenarios on the way.
    disk-guard-accepts-its-configuration =
      pkgs.runCommandLocal "disk-guard-accepts-its-configuration"
        {
          system = workstation.config.environment.etc."disk-guard/config.json".source;
          user = (account workstation).xdg.configFile."disk-guard/config.json".source;
        }
        ''
          touch $out
        '';

    # clippy finds nothing in git-engine, its tests included.
    git-engine-passes-clippy =
      (pkgs.callPackage ./modules/git-engine/package.nix { }).overrideAttrs
        (old: {
          pname = "git-engine-passes-clippy";
          nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
          buildPhase = ''
            cargo clippy --all-targets --offline -- -D warnings
          '';
          doCheck = false;
          installPhase = ''
            touch $out
          '';
        });

    # clippy finds nothing in the backlight daemon, its tests included.
    backlight-passes-clippy =
      (pkgs.callPackage ./modules/backlight/package.nix { }).overrideAttrs
        (old: {
          pname = "backlight-passes-clippy";
          nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
          buildPhase = ''
            cargo clippy --all-targets --offline -- -D warnings
          '';
          doCheck = false;
          installPhase = ''
            touch $out
          '';
        });

    # clippy finds nothing in the clipboard's history, its tests included.
    clipboard-history-passes-clippy =
      (pkgs.callPackage ./modules/quickshell/clipboard/package.nix { }).overrideAttrs
        (old: {
          pname = "clipboard-history-passes-clippy";
          nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
          buildPhase = ''
            cargo clippy --all-targets --offline -- -D warnings
          '';
          doCheck = false;
          installPhase = ''
            touch $out
          '';
        });

    # clippy finds nothing in the window recorder, its tests included.
    record-window-passes-clippy =
      (pkgs.callPackage ./modules/quickshell/record-window/package.nix { }).overrideAttrs
        (old: {
          pname = "record-window-passes-clippy";
          nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
          buildPhase = ''
            cargo clippy --all-targets --offline -- -D warnings
          '';
          doCheck = false;
          installPhase = ''
            touch $out
          '';
        });

    # clippy finds nothing in the console's font tool, its tests included.
    console-font-passes-clippy =
      (pkgs.callPackage ./modules/console/font/package.nix { }).overrideAttrs
        (old: {
          pname = "console-font-passes-clippy";
          nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
          buildPhase = ''
            cargo clippy --all-targets --offline -- -D warnings
          '';
          doCheck = false;
          installPhase = ''
            touch $out
          '';
        });

    # clippy finds nothing in the guard, its scenarios included.
    disk-guard-passes-clippy =
      (pkgs.callPackage ./modules/disk-guard/package.nix { }).overrideAttrs
        (old: {
          pname = "disk-guard-passes-clippy";
          nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
          buildPhase = ''
            cargo clippy --all-targets --offline -- -D warnings
          '';
          doCheck = false;
          installPhase = ''
            touch $out
          '';
        });

    # spend-watch reads the config an account gets: it's refused at build by
    # the watch's own check-config, which builds it and runs its tests on the
    # way, and the service starts the watch with that file.
    spend-watch-accepts-its-configuration =
      let
        home = account workstation;
        file = home.xdg.configFile."spend-watch/config.json".source;
        # Every option set, so each shape the module renders is one the
        # daemon reads: a feed, a cap of the account's own, a zone.
        fed = account (bareNixos [
          self.nixosModules.workstation
          {
            home-manager.users.someone.jitsusama.spend-watch = {
              feeds.provider = {
                label = "the provider";
                command = [
                  "provider-feed"
                  "run"
                ];
                environment.REGION = "north";
              };
              cap.allowance = 4000;
              zone = "America/Toronto";
              premium = [ "big-model" ];
              detectors.quiet = [ "spikes" ];
            };
          }
        ]);
      in
      pkgs.runCommandLocal "spend-watch-accepts-its-configuration"
        {
          inherit file;
          fed = fed.xdg.configFile."spend-watch/config.json".source;
          service = home.systemd.user.services.spend-watch.Service.ExecStart;
        }
        ''
          case "$service" in
            *"--config $file run") ;;
            *)
              echo "The service doesn't start spend-watch with the account's config: $service"
              exit 1
              ;;
          esac
          touch $out
        '';

    # clippy finds nothing in spend-watch, its tests included.
    spend-watch-passes-clippy =
      (pkgs.callPackage ./modules/spend-watch/package.nix { }).overrideAttrs
        (old: {
          pname = "spend-watch-passes-clippy";
          nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
          buildPhase = ''
            cargo clippy --all-targets --offline -- -D warnings
          '';
          doCheck = false;
          installPhase = ''
            touch $out
          '';
        });

    # clippy finds nothing in network-revive, its tests included.
    network-revive-passes-clippy =
      (pkgs.callPackage ./modules/network-revive/package.nix { }).overrideAttrs
        (old: {
          pname = "network-revive-passes-clippy";
          nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
          buildPhase = ''
            cargo clippy --all-targets --offline -- -D warnings
          '';
          doCheck = false;
          installPhase = ''
            touch $out
          '';
        });

    # clippy finds nothing in ir-camera, its tests included.
    ir-camera-passes-clippy =
      (pkgs.callPackage ./modules/ir-camera/package.nix { }).overrideAttrs
        (old: {
          pname = "ir-camera-passes-clippy";
          nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
          buildPhase = ''
            cargo clippy --all-targets --offline -- -D warnings
          '';
          doCheck = false;
          installPhase = ''
            touch $out
          '';
        });

    # The macOS guard builds from the same crate and CI has no Mac, so clippy
    # reads the crate, its scenarios included, as macOS would compile it.
    disk-guard-compiles-for-macos =
      let
        toolchain = (pkgs.extend rust-overlay.overlays.default).rust-bin.stable.latest.minimal.override {
          targets = [ "aarch64-apple-darwin" ];
          extensions = [ "clippy" ];
        };
      in
      (pkgs.callPackage ./modules/disk-guard/package.nix { }).overrideAttrs {
        pname = "disk-guard-compiles-for-macos";
        buildPhase = ''
          export PATH=${toolchain}/bin:$PATH
          cargo clippy --target aarch64-apple-darwin --all-targets --offline -- -D warnings
        '';
        doCheck = false;
        installPhase = ''
          touch $out
        '';
      };

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
