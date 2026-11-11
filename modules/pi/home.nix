# pi, the coding agent in the terminal: a theme drawn from the machine's, so
# its messages, tools, diffs and code wear the same colours as everything
# else. pi keeps which theme it uses in a settings file of its own, so this
# only puts the theme where pi finds it, for Joel to pick in /settings, and
# merges into that file the settings declared for it in jitsusama.pi.settings,
# core's few and a machine's own (settings.nix). Its
# keys come from keys.nix, written where pi reads them and into the key map.
# Its status line tells a folder and git as the prompt does, by the prompt's
# root markers, its git-engine and the theme's marks. pi-claude runs it on
# Joel's own Claude plan (claude.nix), and pi-ask answers the launcher's Ask
# row on the plan a machine names in jitsusama.pi.ask (ask.nix). A personal
# machine sets jitsusama.pi.plan to "claude", and then plain pi is on that
# plan, with no pi-claude beside it: Joel wants one pi that just works.
# On Linux a run Joel typed rings when it's done, with a low bell
# (chime.nix).
# Each morning, on Linux, the installed harness writes its measure in the
# background: what the day cost and which mistakes recurred. A realm a
# machine names in jitsusama.pi.hosts runs its conversations in shared pi
# hosts, started with the session (host.nix).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme)
    appearance
    marks
    colours
    syntax
    ;
  mix = import ../theme/mix.nix { inherit lib; };

  # A tool's result is a faint tint of how it went, under text that stays
  # the text's own colour.
  tint = weight: colour: mix weight colour colours.background;

  # pi's own names for what it colours, in its theme format:
  # https://pi.dev/schemas/theme.schema.json
  theme = {
    name = "jitsusama";
    inherit appearance;
    colors = {
      inherit (colours)
        accent
        text
        success
        warning
        ;
      error = colours.alert;
      border = colours.rule;
      borderAccent = colours.accent;
      borderMuted = colours.edge;
      inherit (colours) muted;
      dim = colours.muted;
      thinkingText = colours.muted;
      selectedBg = colours.selection;
      searchMatchBg = colours.raised;
      searchMatchText = colours.strong;
      scrollbarTrack = colours.raised;
      scrollbarThumb = colours.rule;
      # What Joel wrote sits on the sunken shade, a step below the rest.
      userMessageBg = colours.sunken;
      userMessageText = colours.text;
      customMessageBg = colours.sunken;
      customMessageText = colours.muted;
      customMessageLabel = colours.voice;
      toolPendingBg = colours.sunken;
      toolSuccessBg = tint 0.1 colours.success;
      toolErrorBg = tint 0.12 colours.alert;
      toolTitle = colours.strong;
      toolOutput = colours.muted;
      # Markdown reads as Neovim draws it: headings strong, code as strings,
      # the marks between muted.
      mdHeading = colours.strong;
      mdLink = colours.program;
      mdLinkUrl = colours.muted;
      mdCode = syntax.string.colour;
      mdCodeBlock = colours.text;
      mdCodeBlockBorder = colours.rule;
      mdQuote = colours.muted;
      mdQuoteBorder = colours.rule;
      mdHr = colours.rule;
      mdListBullet = syntax.punctuation.colour;
      toolDiffAdded = colours.success;
      toolDiffRemoved = colours.alert;
      toolDiffContext = colours.muted;
      syntaxComment = syntax.comment.colour;
      syntaxKeyword = syntax.keyword.colour;
      syntaxFunction = syntax.function.colour;
      syntaxVariable = colours.text;
      syntaxString = syntax.string.colour;
      syntaxNumber = syntax.constant.colour;
      syntaxType = syntax.type.colour;
      syntaxOperator = syntax.operator.colour;
      syntaxPunctuation = syntax.punctuation.colour;
      # How hard pi thinks warms from the muted text to the alert, a step
      # through the glyphs' hues at each level.
      thinkingOff = colours.muted;
      thinkingMinimal = syntax.type.colour;
      thinkingLow = colours.program;
      thinkingMedium = colours.voice;
      thinkingHigh = colours.person;
      thinkingXhigh = colours.warning;
      thinkingMax = colours.alert;
      # A shell command is something to do.
      bashMode = colours.action;
    };
    export = {
      pageBg = colours.background;
      cardBg = colours.sunken;
      infoBg = colours.raised;
    };
  };
  keys = import ./keys.nix;

  # The Claude Code the claude module installs, so pi-claude drives the one
  # Joel signs in with, or nixpkgs' own on a machine without that module.
  claude =
    if config.programs.claude-code.package != null then
      config.programs.claude-code.package
    else
      pkgs.claude-code;

  onClaude = config.jitsusama.pi.plan == "claude";

  # Where pi-packages keeps the harness's current version, and the script in
  # it a timer runs.
  measure = "${config.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts/measure.ts";
  restoreFile = "${config.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts/restore-file.ts";
  windowsElsewhere = "${config.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts/windows-elsewhere.ts";
  carryScript = "${config.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts/pi-carry.ts";

  # What mycelium carry runs to carry a quest's trees and sessions to
  # another machine, and to put them back on arriving: the harness's
  # program, run by the store's node.
  piCarry = pkgs.writeShellScriptBin "pi-carry" ''
    exec ${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${carryScript} "$@"
  '';

  # On the Claude plan, the machine's own Claude Code login is the plan's.
  piClaude = import ./claude.nix {
    inherit pkgs claude homeRealm;
    inherit (config.jitsusama.pi.claude) model realm;
    ownLogin = onClaude;
  };

  # The machine's home realm and the further ones it joins, as mycelium
  # says them where its module is loaded.
  homeRealm = lib.attrByPath [ "jitsusama" "mycelium" "realm" ] null config;
  further = lib.attrNames (lib.attrByPath [ "jitsusama" "mycelium" "alsoJoins" ] { } config);

  # Each plan's small, quick model, for answers that take a second or two.
  quickModels = {
    pi = "anthropic/claude-haiku-4-5";
    claude = "claude-code/claude-haiku-4-5-20251001";
  };

  # Each action named in keys.nix, with every chord listed for it.
  keybindings = lib.mapAttrs (_: map (key: key.chord)) (
    lib.groupBy (key: key.action) (
      lib.concatMap (key: map (action: key // { inherit action; }) key.actions) keys
    )
  );
in
{
  imports = [
    ./agents.nix
    ./host.nix
    ../theme/home.nix
    ../keys/home.nix
    ../prompt/home.nix
    ../pi-packages/home.nix
  ];

  options.jitsusama.pi.settings = lib.mkOption {
    type = lib.types.submodule { freeformType = (pkgs.formats.json { }).type; };
    default = { };
    example = {
      defaultProvider = "anthropic";
      packages = [ "git:github.com/Jitsusama/agentic-harness.pi" ];
    };
    description = ''
      Settings merged into pi's own settings file at every switch, each
      winning over what pi wrote there, every other key pi wrote kept. The
      file stays pi's, so a machine sets its settings here rather than
      linking the file, which the merge would replace.
    '';
  };

  options.jitsusama.pi.packages = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [ "git:github.com/someone/their.pi" ];
    description = ''
      The pi packages every pi loads, as pi names their sources: core's own,
      and whatever a machine adds, each definition adding to the others.
      They are the whole of pi's package list, written at every switch, so
      a package installed by hand and not named here is dropped from it.
      Each `git:` one is installed by pi-packages, which follows its branch
      and swaps each new commit in whole, and pi is given its current link.
    '';
  };

  options.jitsusama.pi.chime = lib.mkOption {
    type = lib.types.attrsOf (lib.types.either lib.types.str lib.types.int);
    readOnly = true;
    default = import ./chime.nix { inherit pkgs lib; };
    defaultText = lib.literalExpression "import ./chime.nix { inherit pkgs lib; }";
    description = ''
      What pi rings with on Linux when a run is done, for anything else
      that wants the same loud, low, dog-safe sound: `ding`, a WAV of a
      bell on G3 that dies away over about three seconds; `want`, the
      same bell struck twice, which the shell plays when a session first
      wants Joel; and `player`, PipeWire's pw-play, which plays either as
      `player ding`. Read it rather
      than setting it; a Mac rings with its own sound and has no use for it.
    '';
  };

  options.jitsusama.pi.plan = lib.mkOption {
    type = lib.types.enum (lib.attrNames quickModels);
    default = "pi";
    description = ''
      Which plan every pi runs on: "pi" for pi's own providers, as a work
      machine has them, or "claude" for Joel's own Claude plan through
      Claude Code's login on the machine, never its extra usage. On "claude"
      plain `pi` is all there is: there is no pi-claude beside it.
    '';
  };

  options.jitsusama.pi.claude.model = lib.mkOption {
    type = lib.types.str;
    default = "claude-sonnet-5";
    example = "claude-opus-5";
    description = ''
      The model pi starts on when it runs on Joel's own Claude plan, as
      the claude-code provider names it: plain pi on a machine whose plan
      is "claude", and pi-claude everywhere else. A plan's limits are small,
      so it's a mid-sized model; a larger plan changes only this.
    '';
  };

  options.jitsusama.pi.claude.realm = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "personal";
    description = ''
      The realm pi-claude's sessions belong to, a further realm this
      machine joins for Joel's own work (jitsusama.mycelium.alsoJoins), or
      null to keep them on no mesh. Never the home realm, whose mesh is the
      work one wherever pi-claude is installed; in the further realm a
      session keeps its folders apart from the home realm's.
    '';
  };

  options.jitsusama.pi.ask = {
    plan = lib.mkOption {
      type = lib.types.enum (lib.attrNames quickModels);
      default = config.jitsusama.pi.plan;
      defaultText = lib.literalExpression "config.jitsusama.pi.plan";
      description = ''
        Which plan answers the launcher's Ask row: "pi" for pi's own
        provider, as pi is set up on the machine, or "claude" for Joel's own
        Claude plan through pi-claude's way of starting pi.
      '';
    };
    model = lib.mkOption {
      type = lib.types.str;
      default = quickModels.${config.jitsusama.pi.ask.plan};
      defaultText = lib.literalExpression "the plan's small, quick model";
      description = "The model the Ask row's answers come from, as pi names it.";
    };
    extensions = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "/home/someone/.pi/agent/extensions/a-proxy" ];
      description = ''
        The only pi extensions the Ask row loads, such as the one a work
        machine reaches its models through. Every other is left out, so an
        answer starts in a second or two.
      '';
    };
    person = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = config.jitsusama.identity.name or null;
      defaultText = lib.literalExpression "config.jitsusama.identity.name, where there is one";
      description = ''
        Who the Ask row's answers are for, named in the digest of what's true
        on the machine that each question starts with, or null to name
        nobody.
      '';
    };
    path = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "/home/someone/.local/share/dev/bin" ];
      description = ''
        Folders put ahead of the PATH the Ask row's pi starts with, in this
        order, for what its extensions run, such as the program a work
        machine's proxy gets its token from. The launcher starts it from a
        user service, whose PATH isn't a login shell's.
      '';
    };
  };

  config = {
    assertions = [
      {
        # A further realm with a home beside it: with no home realm named,
        # the harness would take this one for home and write into its folders.
        assertion =
          config.jitsusama.pi.claude.realm != null
          ->
            homeRealm != null
            && config.jitsusama.pi.claude.realm != homeRealm
            && lib.elem config.jitsusama.pi.claude.realm further;
        message = "jitsusama.pi.claude.realm names no realm of jitsusama.mycelium.alsoJoins.";
      }
    ];

    home.file.".pi/agent/themes/jitsusama.json".text = builtins.toJSON theme;

    # What pi compiles its TypeScript into, shared by every pi here. Each
    # install of a package compiles its changed files afresh and nothing
    # removes the old ones: by 6 November it held 265,000 files in 2.1 GB.
    # Files go two days after they were made, since /tmp keeps no read
    # times; one still in use costs a compile the next time it loads.
    systemd.user.tmpfiles.rules = lib.mkIf pkgs.stdenv.hostPlatform.isLinux [
      "e /tmp/jiti - - - 2d"
    ];

    # landrun fences a session's commands with Landlock when it starts with
    # PI_SANDBOX=strict, as one reading a stranger's text should; without it
    # such a session runs no command at all. bubblewrap goes around it for
    # the agent fence: a pid namespace of its own, so no other process's
    # /proc, and a runtime folder holding only the sockets pi lets through,
    # so no display, bus or password socket. Nothing runs under either
    # until a session asks for a fence.
    home.packages =
      lib.optional (!onClaude) piClaude
      ++ [
        (import ./ask.nix {
          inherit pkgs lib piClaude;
          inherit (config.jitsusama.pi) ask;
        })
      ]
      ++ [ piCarry ]
      ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [
        pkgs.landrun
        pkgs.bubblewrap
      ];

    # On the Claude plan every pi starts on it: the provider is one of the
    # extensions pi finds by itself, so the package list pi keeps is left
    # alone, and Sonnet is where it starts, a plan's limits being small.
    home.file.".pi/agent/extensions/claude-code" = lib.mkIf onClaude {
      source = "${import ./claude-provider.nix { inherit pkgs; }}/src";
    };
    home.sessionVariables = lib.mkIf onClaude {
      PI_CLAUDE_CODE_COMMAND = lib.getExe claude;
    };
    #
    # Off the plan every pi starts on Opus 5.5 at high thinking, the one
    # model its sessions run on. pi rewrites these keys when a model is
    # picked as the default, and a launcher pi is wrapped in may write its
    # own: on 9 November one moved every new session to another model.
    # Each switch puts them back.
    jitsusama.pi.settings.defaultProvider = lib.mkMerge [
      (lib.mkIf onClaude "claude-code")
      (lib.mkDefault "anthropic")
    ];
    jitsusama.pi.settings.defaultModel = lib.mkMerge [
      (lib.mkIf onClaude config.jitsusama.pi.claude.model)
      (lib.mkDefault "claude-opus-5-5")
    ];
    jitsusama.pi.settings.defaultThinkingLevel = lib.mkDefault "high";

    # The harness, and neovim.pi for pi beside Neovim, on every machine,
    # first, since pi loads its packages in order.
    jitsusama.pi.packages = lib.mkBefore [
      "git:github.com/Jitsusama/agentic-harness.pi"
      "git:github.com/Jitsusama/neovim.pi"
    ];
    jitsusama.pi-packages.sources = config.jitsusama.pi.packages;
    jitsusama.pi.settings.packages = config.jitsusama.pi-packages.links;

    # The room kept for what a compaction writes. pi's default, 16,384
    # tokens, runs out summarising a long session, which then compacts
    # nothing and runs on toward the model's limit.
    jitsusama.pi.settings.compaction.reserveTokens = lib.mkDefault 64000;

    # The recent conversation a compaction keeps whole, as pi estimates it,
    # at four characters a token and with only the thinking it shows. On 3
    # November, in five of one session's compactions, what pi's default of
    # 20,000 kept was 35,265 to 42,137 tokens as the model read them, about
    # twice, and every turn after re-reads it until the next compaction;
    # that day's 234 compactions landed at 126k on average. Half the estimate keeps about what the
    # default meant to.
    jitsusama.pi.settings.compaction.keepRecentTokens = lib.mkDefault 10000;

    # Links pi draws, which a click opens. pi turns them on only when it can
    # tell it runs in kitty, by KITTY_WINDOW_ID or TERM_PROGRAM, and a
    # session held apart from its window has neither; pi then draws no
    # links, and in fullscreen it takes every click, so a click opens nothing.
    jitsusama.pi.settings.terminal.hyperlinks = lib.mkDefault true;

    home.activation.piTakesCoresSettings =
      lib.hm.dag.entryAfter
        [
          "writeBoundary"
          "piPackagesInstall"
        ]
        ''
          run ${
            lib.getExe (
              import ./settings.nix {
                inherit pkgs;
                inherit (config.jitsusama.pi) settings;
              }
            )
          } ${lib.escapeShellArg "${config.home.homeDirectory}/.pi/agent/settings.json"}
        '';

    # pi writes its session logs readable by anyone on the machine, and a
    # tool's output in them can hold a token: on 7 November 84 of 625 logs
    # did. pi's own folder is kept to its account alone, which closes every
    # file under it whatever mode pi writes it with; a umask for pi would
    # also make every file an agent writes private, which this doesn't.
    home.activation.piKeepsItsFolderPrivate = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      run mkdir -p ${lib.escapeShellArg "${config.home.homeDirectory}/.pi/agent/sessions"}
      run chmod 0700 ${lib.escapeShellArg "${config.home.homeDirectory}/.pi/agent"} \
        ${lib.escapeShellArg "${config.home.homeDirectory}/.pi/agent/sessions"}
    '';

    # Where the harness's status line learns the folder and git; its README
    # says what each field means.
    xdg.configFile."pi/agentic-harness.pi/status-line/where.json".text = builtins.toJSON {
      engine = lib.getExe (pkgs.callPackage ../git-engine/package.nix { });
      inherit (config.jitsusama.prompt) roots;
      marks = lib.mapAttrs (_: mark: mark.glyph) {
        inherit (marks)
          branch
          more
          complete
          active
          pending
          thinking
          ;
      };
    };

    # What a run Joel typed rings with when it's done, where the harness's
    # done-chime-widget reads it: the ding and what plays it, and no voice
    # (chime.nix). A Mac rings with its own sound, which the harness knows.
    xdg.configFile."pi/agentic-harness.pi/done-chime-widget/sound.json" =
      lib.mkIf pkgs.stdenv.hostPlatform.isLinux
        {
          text = builtins.toJSON { inherit (config.jitsusama.pi.chime) ding player; };
        };

    home.file.".pi/agent/keybindings.json".text = builtins.toJSON (
      { "$schema" = "https://pi.dev/schemas/keybindings.schema.json"; } // keybindings
    );

    # The harness's measure: its cost audit of the last day against the day
    # before, and its learning sensor over the last seven days day by day,
    # as one report in its state folder's measure/. A day rather than a
    # week, so a harness change shows the day after it lands. pi-packages
    # installs the harness, so a machine without it, or before its first
    # install, skips the run rather than failing it. The run reads the day's session logs, so it waits behind
    # whatever the account is doing.
    systemd.user.services.pi-measure = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "Writes the pi harness's daily measure";
        ConditionPathExists = measure;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe pkgs.nodejs} ${measure}";
        Slice = "background.slice";
      };
    };
    systemd.user.timers.pi-measure = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Writes the pi harness's measure every morning";
      Timer = {
        OnCalendar = "*-*-* 06:00:00";
        RandomizedDelaySec = "10min";
        # A morning slept through runs once the machine wakes.
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
    };

    # The launcher's list of the sessions the last boot's end took away,
    # written as the account's session starts, before any pi has: each pi
    # rewrites it as it starts, but until one does the file would still
    # describe an older boot. It reads only the harness's session records,
    # and a machine without the harness skips it.
    systemd.user.services.pi-restore-file = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "Writes which pi sessions the last boot's end took away";
        ConditionPathExists = restoreFile;
      };
      Service = {
        Type = "oneshot";
        # The harness's package.json names no module type, on purpose: two
        # of its modules load with require. Node warns about that on every
        # run, which said nothing at each switch.
        ExecStart = "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${restoreFile}";
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };

    # A session another machine holds runs on through this machine's
    # power-off, so what the boot takes away is only the window here
    # joined to it, and only this machine knows it was here. Each minute
    # the harness notes those windows, from kitty's and the mycelium
    # node's answers, so the next boot's list can offer them back.
    systemd.user.services.pi-windows-elsewhere = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "Notes the windows joined to sessions other machines hold";
        ConditionPathExists = windowsElsewhere;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${windowsElsewhere}";
        Slice = "background.slice";
      };
    };
    systemd.user.timers.pi-windows-elsewhere = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Notes each minute the windows joined to sessions other machines hold";
      Timer = {
        OnCalendar = "minutely";
        AccuracySec = "10s";
      };
      Install.WantedBy = [ "timers.target" ];
    };

    jitsusama.keys.pi = map (key: {
      inherit (key) chord does;
      when = key.when or "";
    }) keys;
  };
}
