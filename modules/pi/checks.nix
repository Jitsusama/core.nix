# The checks of pi, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
args@{
  account,
  bareNixos,
  holds,
  lib,
  macWorkstation,
  passesClippy,
  pkgs,
  self,
  workstation,
  ...
}:
let
  # pi as a host runs it, every host patch applied exactly (hosted-pi.nix).
  hostedPi = import ./hosted-pi.nix {
    inherit pkgs;
    folder = "pi-monorepo";
  };
in
# The hosts' own checks change with host.nix alone, so they sit beside it.
import ./host-checks.nix args
// {
  # pi's folder, its session logs among what it holds, is the account's
  # alone, set as the account is switched to, on every machine with pi.
  pi-keeps-its-folder-private =
    let
      home = account workstation;
      mac = account macWorkstation;
      keeps = account: account.home.activation.piKeepsItsFolderPrivate.data or "";
      closes = script: lib.hasInfix "chmod 0700" script && lib.hasInfix "/.pi/agent/sessions" script;
    in
    holds "pi-keeps-its-folder-private"
      "pi's folder or its session logs' folder is left readable by other accounts on a machine with pi."
      (closes (keeps home) && closes (keeps mac));

  # pi-carry is on the account's path on every machine with pi, running the
  # harness's own program with the store's node, for mycelium carry.
  pi-carry-is-on-the-path =
    let
      carries = home: lib.any (package: lib.getName package == "pi-carry") home.home.packages;
    in
    holds "pi-carry-is-on-the-path"
      "pi-carry, which mycelium carry runs, isn't on the path of an account with pi."
      (carries (account workstation) && carries (account macWorkstation));

  # The harness's measure runs each morning in the background, catching up
  # on a morning slept through, with the store's node, and only where the
  # harness is installed; a Mac, with no systemd, gets none.
  pi-measures-each-day-in-the-background =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-measure;
      timer = home.systemd.user.timers.pi-measure.Timer;
      # home-manager keeps a unit's settings as lists.
      first = value: lib.head (lib.toList value);
    in
    holds "pi-measures-each-day-in-the-background"
      "pi's measure runs outside background.slice, not every day, misses a morning slept through, runs a node off the PATH, fails where the harness isn't installed, or reaches a Mac."
      (
        service.Service.Slice == "background.slice"
        && timer.Persistent
        && lib.hasPrefix "*-*-* " (first timer.OnCalendar)
        && lib.hasPrefix "${lib.getExe pkgs.nodejs} " (first service.Service.ExecStart)
        && lib.hasSuffix "/scripts/measure.ts" (first service.Unit.ConditionPathExists)
        && !(mac.systemd.user.services ? pi-measure)
      );

  # The launcher's list of what the last boot's end took away is written as
  # the account's session starts, with the store's node, only where the
  # harness is installed; a Mac, with no systemd, gets none.
  pi-writes-what-the-last-boot-left =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-restore-file;
      first = value: lib.head (lib.toList value);
    in
    holds "pi-writes-what-the-last-boot-left"
      "pi's list of the sessions the last boot's end took away isn't written as the session starts, runs outside background.slice or a node off the PATH, logs node's module-type warning, fails where the harness isn't installed, or reaches a Mac."
      (
        lib.elem "default.target" service.Install.WantedBy
        && service.Service.Slice == "background.slice"
        && lib.hasPrefix "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON " (
          first service.Service.ExecStart
        )
        && lib.hasSuffix "/scripts/restore-file.ts" (first service.Unit.ConditionPathExists)
        && !(mac.systemd.user.services ? pi-restore-file)
      );

  # The windows here joined to another machine's session are noted each
  # minute, quietly in the background, only where the harness is installed,
  # so the next boot can offer them back; a Mac gets none.
  pi-notes-windows-on-sessions-elsewhere =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-windows-elsewhere;
      timer = home.systemd.user.timers.pi-windows-elsewhere;
      first = value: lib.head (lib.toList value);
    in
    holds "pi-notes-windows-on-sessions-elsewhere"
      "pi doesn't note each minute the windows joined to sessions other machines hold, runs outside background.slice or a node off the PATH, logs node's module-type warning, fails where the harness isn't installed, or reaches a Mac."
      (
        lib.elem "timers.target" timer.Install.WantedBy
        && first timer.Timer.OnCalendar == "minutely"
        && service.Service.Slice == "background.slice"
        && lib.hasPrefix "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON " (
          first service.Service.ExecStart
        )
        && lib.hasSuffix "/scripts/windows-elsewhere.ts" (first service.Unit.ConditionPathExists)
        && !(mac.systemd.user.timers ? pi-windows-elsewhere)
      );

  # The launcher's AI actions are served on a socket systemd holds beside
  # the other daemons', named as daemons are, restarted by a switch, left
  # to rest once idle, in the background, only where the harness is
  # installed; a Mac gets none.
  pi-serves-the-launchers-agent-actions =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-verbs;
      socket = home.systemd.user.sockets.pi-verbs;
      first = value: lib.head (lib.toList value);
    in
    holds "pi-serves-the-launchers-agent-actions"
      "The launcher's AI actions aren't served on daemons/pi-verbs.sock held by systemd for the login alone, aren't named pi-verbs, outlive a switch, are started again after a rest, run outside background.slice or a node off the PATH, fail where the harness isn't installed, or reach a Mac."
      (
        socket.Socket.ListenStream == "%t/daemons/pi-verbs.sock"
        && socket.Socket.SocketMode == "0600"
        && lib.elem "sockets.target" socket.Install.WantedBy
        && service.Unit.Requires == [ "pi-verbs.socket" ]
        && service.Unit.X-SwitchMethod == "restart"
        && service.Service.Slice == "background.slice"
        && service.Service.Restart == "on-failure"
        && lib.hasPrefix "@" (first service.Service.ExecStart)
        && lib.hasSuffix "/bin/pi-verbs pi-verbs serve" (first service.Service.ExecStart)
        && lib.any (package: lib.getName package == "pi-verbs") home.home.packages
        && lib.hasSuffix "/scripts/pi-verbs.ts" (first service.Unit.ConditionPathExists)
        && lib.hasSuffix "/scripts/pi-verbs.ts" (first socket.Unit.ConditionPathExists)
        && !(mac.systemd.user.services ? pi-verbs)
        && !(mac.systemd.user.sockets ? pi-verbs)
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
      theme.colors.text == home.jitsusama.theme.colours.text
      && theme.colors.syntaxString == home.jitsusama.theme.syntax.string.colour
      && theme.colors.thinkingMax == home.jitsusama.theme.colours.alert
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
      inherit (home.jitsusama.theme) marks;
      engine = lib.getExe (pkgs.callPackage ../git-engine/package.nix { });
    in
    holds "pi-tells-where-as-the-prompt-does"
      "pi's status line tells the folder or git by something other than the prompt's roots, engine and marks."
      (
        where.roots == home.jitsusama.prompt.roots
        && where.engine == builtins.unsafeDiscardStringContext engine
        &&
          where.marks == {
            branch = marks.branch.glyph;
            more = marks.more.glyph;
            complete = marks.complete.glyph;
            active = marks.active.glyph;
            pending = marks.pending.glyph;
            thinking = marks.thinking.glyph;
          }
      );

  # A run Joel typed rings with what chime.nix names, in the file the
  # harness's done-chime-widget reads, on Linux alone: the store's pw-play
  # and a ding, and no voice, since Joel kept only the bell on 9 November.
  # It names the shell's Do Not Disturb file, under the runtime folder,
  # which holds the ring while it's on. A Mac rings with its own sound, so
  # it gets no file.
  pi-rings-when-a-run-is-done =
    let
      home = account workstation;
      mac = account macWorkstation;
      file = "pi/agentic-harness.pi/done-chime-widget/sound.json";
      sound = builtins.fromJSON (builtins.unsafeDiscardStringContext home.xdg.configFile.${file}.text);
      store = path: builtins.unsafeDiscardStringContext path;
    in
    holds "pi-rings-when-a-run-is-done"
      "pi's ring names no ding or quiet file, another player, a voice, or reaches a Mac."
      (
        lib.hasPrefix builtins.storeDir sound.ding
        && sound.player == store (lib.getExe' pkgs.pipewire "pw-play")
        && (sound.quiet or null) == "quiet"
        && !(sound ? voice)
        && !(sound ? want)
        && !(mac.xdg.configFile ? ${file})
      );

  # A session wanting Joel rings pi's low bell, struck twice, in the shell
  # of a machine with pi, rather than the theme's high message. pi comes
  # with a workstation and the shell with a screen, so the machine is both.
  a-want-rings-low =
    let
      home = account (bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
      ]);
    in
    holds "a-want-rings-low"
      "The shell of a machine with pi plays something other than pi's want sound when a session wants Joel."
      (toString home.jitsusama.quickshell.sounds.wants == home.jitsusama.pi.chime.want);

  # The account's ding rings for seconds, loud, as the harness plays it,
  # and the shell's want sound rings loud, both low.
  pi-rings-low =
    let
      home = account workstation;
      sound =
        pkgs.writeText "sound.json"
          home.xdg.configFile."pi/agentic-harness.pi/done-chime-widget/sound.json".text;
    in
    pkgs.runCommand "pi-rings-low"
      {
        nativeBuildInputs = [
          pkgs.jq
          pkgs.sox
        ];
      }
      ''
        ${lib.getExe pkgs.bash} ${./tests/pi-rings-low.sh} ${sound} ${home.jitsusama.pi.chime.want}
        touch $out
      '';

  # pi's settings file gets the settings declared for it at every switch,
  # core's and a machine's own, and keeps every key pi wrote there; a file
  # that isn't JSON is left alone and the switch says so; and a file already
  # holding them isn't written again.
  pi-keeps-its-own-settings =
    let
      home = account workstation;
      program = lib.getExe (
        import ./settings.nix {
          inherit pkgs;
          inherit (home.jitsusama.pi) settings;
        }
      );
      # As a machine adds its own, such as the model it starts with.
      machines = lib.getExe (
        import ./settings.nix {
          inherit pkgs;
          settings = home.jitsusama.pi.settings // {
            defaultModel = "a-model";
          };
        }
      );
      # The same program as the workstation builds it: the workstation is
      # always Linux, and a check run on a Mac builds the one it runs for
      # the Mac, at another path.
      activated = lib.getExe (
        import ./settings.nix {
          inherit (workstation) pkgs;
          inherit (home.jitsusama.pi) settings;
        }
      );
    in
    assert lib.assertMsg (lib.hasInfix (builtins.unsafeDiscardStringContext activated) (
      builtins.unsafeDiscardStringContext home.home.activation.piTakesCoresSettings.data
    )) "pi's activation doesn't run the settings program the check tests.";
    pkgs.runCommandLocal "pi-keeps-its-own-settings" { nativeBuildInputs = [ pkgs.jq ]; } ''
      ${lib.getExe pkgs.bash} ${./tests/pi-keeps-its-own-settings.sh} ${program} ${machines}
      touch $out
    '';

  # pi-claude starts pi on the plan alone: the provider, Sonnet unless told
  # otherwise, off the mesh unless the machine joins a further realm for the
  # plan, and nothing that would bill an API instead; and an account gets it.
  pi-claude-keeps-to-the-plan =
    let
      home = account workstation;
      # Claude Code as pi-claude starts it, writing down how.
      claude = pkgs.writeShellScriptBin "claude" ''
        printf 'arg %s\n' "$@" >> "$SAID"
        env | sed 's/^/env /' >> "$SAID"
      '';
      # The plan's model as an account has it.
      inherit (home.jitsusama.pi.claude) model;
      program = lib.getExe (import ./claude.nix { inherit pkgs claude model; });
      onOwnLogin = lib.getExe (
        import ./claude.nix {
          inherit pkgs claude model;
          ownLogin = true;
        }
      );
      inOther = lib.getExe (
        import ./claude.nix {
          inherit pkgs claude model;
          realm = "other";
          homeRealm = "home";
        }
      );
      # An account naming pi-claude's realm, with or without joining it.
      naming =
        joins:
        account (
          workstation.extendModules {
            modules = [
              {
                home-manager.users.someone.jitsusama = {
                  mycelium.realm = "home";
                  mycelium.alsoJoins = lib.optionalAttrs joins { other = { }; };
                  pi.claude.realm = "other";
                };
              }
            ];
          }
        );
      refused =
        home:
        lib.any (
          check: !check.assertion && lib.hasInfix "jitsusama.pi.claude.realm" check.message
        ) home.assertions;
    in
    assert lib.assertMsg (lib.any (
      package: lib.getName package == "pi-claude"
    ) home.home.packages) "An account with pi doesn't get pi-claude.";
    assert lib.assertMsg (
      refused (naming false) && !refused (naming true)
    ) "pi-claude's realm isn't held to a further realm the machine joins.";
    pkgs.runCommandLocal "pi-claude-keeps-to-the-plan" { } ''
      ${lib.getExe pkgs.bash} ${./tests/pi-claude-keeps-to-the-plan.sh} ${program} \
        ${lib.getExe claude} ${onOwnLogin} ${inOther}
      touch $out
    '';

  # A used-up Claude plan limit is said plainly, by the provider as it is
  # patched: which limit, when it resets, and that nothing went to extra
  # usage, in words pi never retries.
  pi-says-when-the-plan-runs-out = pkgs.runCommandLocal "pi-says-when-the-plan-runs-out" { } ''
    ${lib.getExe pkgs.nodejs} --experimental-strip-types --no-warnings \
      ${./tests/pi-says-when-the-plan-runs-out.mjs} ${import ./claude-provider.nix { inherit pkgs; }}
    touch $out
  '';

  # A hosted pi's seams are there to detect, and a turn its durable generation
  # commits is the host's to end or continue.
  pi-hosted-keeps-turns-its-own = pkgs.runCommandLocal "pi-hosted-keeps-turns-its-own" { } ''
    HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
      ${./tests/pi-hosted-keeps-turns-its-own.mjs} ${hostedPi}/lib/node_modules/pi-monorepo
    touch $out
  '';

  # A hosted pi loads a linked extension through where its link points now,
  # so once the link an extension is found through moves, a reload loads the
  # new copy and its own helpers, whether the old copy is gone or not.
  pi-hosted-loads-where-a-link-points =
    pkgs.runCommandLocal "pi-hosted-loads-where-a-link-points" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-loads-where-a-link-points.mjs} ${hostedPi}/lib/node_modules/pi-monorepo
        touch $out
      '';

  # A hosted pi's conversations each read their keys and headers from their
  # own auth context and never from the process, while a pi given none
  # reads the process as ordinary pi does.
  pi-hosted-keeps-each-conversations-keys =
    pkgs.runCommandLocal "pi-hosted-keeps-each-conversations-keys" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-keeps-each-conversations-keys.mjs} ${hostedPi}/lib/node_modules/pi-monorepo
        touch $out
      '';

  # A hosted pi's screen stays in its own conversation: quitting or a crash
  # ends only that screen and what it was doing, a stopped screen builds
  # nothing for an extension and disposes the footer, header and editor one
  # gave it, a suspend stops only the viewer that asked, every screen hears
  # a theme change, and ordinary pi keeps every path.
  pi-hosted-screen-stays-in-its-session =
    pkgs.runCommandLocal "pi-hosted-screen-stays-in-its-session" { }
      ''
        HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-screen-stays-in-its-session.mjs} ${hostedPi}/lib/node_modules/pi-monorepo
        touch $out
      '';

  # A hosted pi's screen does in its viewers' terminals what pi does in its
  # own: a copy reaches them, a viewer who joins late is sent the images on
  # the screen, every frame is drawn within its terminal's render scope, so
  # as the conversation it shows, the external editor opens where the viewer
  # who asked is, a quit asks its host first, with the key that asked still
  # in hand, and
  # an extension's own component says it waits on the person, so its host
  # can let a viewer attach to answer it.
  pi-hosted-screen-reaches-its-viewers =
    pkgs.runCommandLocal "pi-hosted-screen-reaches-its-viewers" { }
      ''
        HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-screen-reaches-its-viewers.mjs} \
          ${hostedPi}/lib/node_modules/pi-monorepo
        touch $out
      '';

  # pi-ask's digest says what's true on the machine as each question starts,
  # from the machine's own files alone, leaving out what it doesn't have.
  pi-ask-knows-the-machine = pkgs.runCommandLocal "pi-ask-knows-the-machine" { } ''
    ${lib.getExe pkgs.nodejs} --experimental-strip-types --no-warnings \
      ${./tests/pi-ask-knows-the-machine.mjs} ${./ask-digest.ts}
    touch $out
  '';

  # pi-ask's one tool lists the launcher's places and actions and runs one,
  # passing on what was done, or that a disruptive one waits for Return.
  pi-ask-runs-the-launcher = pkgs.runCommandLocal "pi-ask-runs-the-launcher" { } ''
    ${lib.getExe pkgs.nodejs} --experimental-strip-types --no-warnings \
      ${./tests/pi-ask-runs-the-launcher.mjs} ${./ask-launcher.ts}
    touch $out
  '';

  # The patched provider starts Claude Code to ask for its login once for any
  # number of asks within a minute, and asks again while it isn't logged in.
  pi-asks-claude-code-its-login-once =
    pkgs.runCommandLocal "pi-asks-claude-code-its-login-once" { }
      ''
        ${lib.getExe pkgs.nodejs} --experimental-transform-types --no-warnings \
          ${./tests/pi-asks-claude-code-its-login-once.mjs} ${
            import ./claude-provider.nix { inherit pkgs; }
          }
        touch $out
      '';

  # The patched provider hands pi a call to one of pi's tools that isn't
  # loaded, for pi to answer as not found, and a broken relayed stream says
  # what broke it.
  pi-answers-a-tool-it-has-not-loaded =
    pkgs.runCommandLocal "pi-answers-a-tool-it-has-not-loaded" { }
      ''
        ${lib.getExe pkgs.nodejs} --experimental-transform-types --no-warnings \
          ${./tests/pi-answers-a-tool-it-has-not-loaded.mjs} ${
            import ./claude-provider.nix { inherit pkgs; }
          }
        touch $out
      '';

  # A machine on the Claude plan has one pi, on it: the provider is an
  # extension pi finds by itself, pi starts on it and on Sonnet, Claude Code
  # is the one it drives, and there is no pi-claude beside it. The Ask row
  # follows, on the machine's own login. A work machine has none of this.
  # The plan's model is said once, so naming another moves both pis to it.
  pi-runs-on-the-machines-plan =
    let
      onClaude = account (
        workstation.extendModules {
          modules = [ { home-manager.users.someone.jitsusama.pi.plan = "claude"; } ];
        }
      );
      onPi = account workstation;
      onAnother = account (
        workstation.extendModules {
          modules = [
            {
              home-manager.users.someone.jitsusama.pi = {
                plan = "claude";
                claude.model = "a-model";
              };
            }
          ];
        }
      );
      names = home: map lib.getName home.home.packages;
      provider = onClaude.home.file.".pi/agent/extensions/claude-code".source;
    in
    holds "pi-runs-on-the-machines-plan"
      "A machine's pi isn't on the plan it names, or a work machine's pi changed."
      (
        builtins.pathExists "${provider}/index.ts"
        && onClaude.jitsusama.pi.settings.defaultProvider == "claude-code"
        && onClaude.jitsusama.pi.settings.defaultModel == "claude-sonnet-5"
        && onAnother.jitsusama.pi.settings.defaultModel == "a-model"
        && onClaude.home.sessionVariables ? PI_CLAUDE_CODE_COMMAND
        && !(lib.elem "pi-claude" (names onClaude))
        && onClaude.jitsusama.pi.ask.plan == "claude"
        && !(onPi.home.file ? ".pi/agent/extensions/claude-code")
        && onPi.jitsusama.pi.settings.defaultProvider != "claude-code"
        && lib.elem "pi-claude" (names onPi)
        && onPi.jitsusama.pi.ask.plan == "pi"
      );

  # Off the Claude plan pi starts on Opus 5.5 at high thinking, on Linux and
  # on a Mac: the settings every switch writes into pi's file say so.
  pi-starts-on-opus =
    let
      starts =
        home:
        let
          inherit (home.jitsusama.pi) settings;
        in
        settings.defaultProvider == "anthropic"
        && settings.defaultModel == "claude-opus-5-5"
        && settings.defaultThinkingLevel == "high";
    in
    holds "pi-starts-on-opus"
      "pi off the Claude plan doesn't start on anthropic/claude-opus-5-5 at high thinking."
      (starts (account workstation) && starts (account macWorkstation));

  # pi starts without a warning that the harness's codemode took the
  # built-in's place: the settings turn the built-in off, on Linux and on a
  # Mac.
  pi-leaves-out-the-built-in-codemode =
    let
      leaves = home: home.jitsusama.pi.settings.extensions == [ "-builtin:codemode" ];
    in
    holds "pi-leaves-out-the-built-in-codemode"
      "pi's settings don't turn off the built-in codemode, so pi warns of it at every start."
      (leaves (account workstation) && leaves (account macWorkstation));

  # pi tells mycelium what its sessions start with in each realm: plain pi
  # at home, pi-claude in the plan's realm, plain pi alone on a machine on
  # the plan, and nothing for a machine in no realm.
  pi-says-what-starts-its-sessions =
    let
      agents =
        settings:
        (account (
          workstation.extendModules {
            modules = [ { home-manager.users.someone.jitsusama = settings; } ];
          }
        )).jitsusama.mycelium.agents;
    in
    holds "pi-says-what-starts-its-sessions"
      "pi doesn't tell mycelium plain pi at home and pi-claude in the plan's realm, tells pi-claude on the plan, or tells a machine in no realm anything."
      (
        agents {
          mycelium.realm = "home";
          mycelium.alsoJoins.other = { };
          pi.claude.realm = "other";
        } == {
          home = "pi";
          other = "pi-claude";
        }
        &&
          agents {
            mycelium.realm = "personal";
            pi.plan = "claude";
          } == {
            personal = "pi";
          }
        && agents { } == { }
      );

  # pi's package list is declared: core's two on every machine, in order, and
  # a machine's own after them rather than in their place.
  pi-loads-its-packages =
    let
      own = "git:github.com/someone/their.pi";
      plain = account workstation;
      added = account (
        workstation.extendModules {
          modules = [ { home-manager.users.someone.jitsusama.pi.packages = [ own ]; } ];
        }
      );
      core = [
        "git:github.com/Jitsusama/agentic-harness.pi"
        "git:github.com/Jitsusama/neovim.pi"
      ];
      # pi is given each as pi-packages' link to its current release.
      linked =
        home: map (source: "${home.jitsusama.pi-packages.root}/${lib.removePrefix "git:" source}/current");
    in
    holds "pi-loads-its-packages"
      "pi's package list isn't core's two, a machine's package replaced them, or pi is given a git source to update in place rather than its current link."
      (
        plain.jitsusama.pi.packages == core
        && plain.jitsusama.pi.settings.packages == linked plain core
        && lib.take 2 added.jitsusama.pi.settings.packages == linked added core
        && lib.elem (lib.head (linked added [ own ])) added.jitsusama.pi.settings.packages
      );

  # pi-ask starts pi for the Ask row with nothing loaded but the machine's
  # extensions, on the machine's plan and model: pi's own by default, with
  # its quick model, or pi-claude's; started as a user service starts it, it
  # puts the machine's folders first on pi's PATH; and an account gets it.
  pi-ask-answers-on-the-machines-plan =
    let
      home = account workstation;
      # pi-claude as pi-ask starts it, writing down how.
      piClaude = pkgs.writeShellScriptBin "pi-claude" ''
        { echo "started pi-claude"; printf 'arg %s\n' "$@"; } > "$SAID"
      '';
      asking =
        ask:
        lib.getExe (
          import ./ask.nix {
            inherit
              pkgs
              lib
              piClaude
              ask
              ;
          }
        );
      # The workstation's defaults, with a machine's extensions.
      onPi = asking (
        home.jitsusama.pi.ask
        // {
          extensions = [
            "/extensions/a-proxy"
            "/extensions/another"
          ];
          path = [
            "/folders/the-token-maker"
            "/folders/another"
          ];
        }
      );
      onClaude = asking {
        plan = "claude";
        model = "claude-code/claude-haiku-4-5-20251001";
        extensions = [ ];
        path = [ ];
        person = null;
      };
    in
    assert lib.assertMsg (lib.any (
      package: lib.getName package == "pi-ask"
    ) home.home.packages) "An account with pi doesn't get pi-ask.";
    assert lib.assertMsg (
      home.jitsusama.pi.ask.plan == "pi"
    ) "pi-ask's plan isn't pi's own by default.";
    pkgs.runCommandLocal "pi-ask-answers-on-the-machines-plan" { } ''
      ${lib.getExe pkgs.bash} ${./tests/pi-ask-answers-on-the-machines-plan.sh} \
        ${onPi} ${onClaude} ${lib.getExe piClaude}
      touch $out
    '';

  # pi-spawner builds with its tests run (spawner/tests/spawner.rs): what a
  # command says and how it ends passed on, its folder and variables kept,
  # a group of its own, a failure to start said, a drop honoured, and every
  # group killed once pi goes.
  pi-spawner-starts-commands-for-pi = pkgs.callPackage ./spawner/package.nix { };

  # clippy finds nothing in pi-spawner, its tests included.
  pi-spawner-passes-clippy = passesClippy "pi-spawner-passes-clippy" (
    pkgs.callPackage ./spawner/package.nix { }
  );
}
