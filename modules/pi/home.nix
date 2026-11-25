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
# On Linux the installed harness's own scripts run on a schedule or at
# login, its morning measure and its sweep of ended work trees among them
# (timers/), and its closed session logs are rewritten compressed each
# night (timers/compressed-logs.nix). A realm a machine names in
# jitsusama.pi.hosts runs its conversations in shared pi hosts, started
# with the session (host.nix).
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

  # Where pi-packages keeps the harness's current version, and the scripts
  # in it that programs here run.
  carryScript = "${config.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts/pi-carry.ts";
  verbsScript = "${config.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts/pi-verbs.ts";

  # What mycelium carry runs to carry a quest's trees and sessions to
  # another machine, and to put them back on arriving: the harness's
  # program, run by the store's node.
  # The launcher's AI actions, served and asked by the harness's program
  # under the name its socket and units take, as every daemon is shown.
  piVerbs = pkgs.writeShellScriptBin "pi-verbs" ''
    exec -a pi-verbs ${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${verbsScript} "$@"
  '';

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

  # The door the fence's gh and git reach GitHub through, on every Linux
  # machine, since the fence runs on each.
  githubDoor = pkgs.callPackage ./github-door/package.nix { };

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
    ./agent-proxy/home.nix
    ./agents.nix
    ./compile-cache.nix
    ./fence.nix
    ./harness-settings.nix
    ./host.nix
    ./state-subvolumes.nix
    ./timers
    ./timers/compressed-logs.nix
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
    # until a session asks for a fence. pi-spawner starts the agent's
    # commands for the harness, from a process small enough that starting
    # one doesn't stall pi (spawner/src/main.rs). a-call-is-on tells the
    # harness's cache warms to wait out a call, since each sends a whole
    # conversation up the line the call needs; it goes on the account's
    # PATH, which terminal sessions and the pi hosts both search.
    home.packages =
      lib.optional (!onClaude) piClaude
      ++ [ (pkgs.callPackage ./spawner/package.nix { }) ]
      ++ [ (pkgs.callPackage ../calls/package.nix { }) ]
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
        piVerbs
        # On PATH because a fenced command starts the door's relay itself,
        # and the fence keeps the account's PATH.
        githubDoor
      ];

    # The door a fenced gh or git reaches GitHub through, since the token
    # lives in the keyring the fence hides. The harness asks it over the
    # control socket, which the fence never lets in, for a session's own
    # folder; the fence lets in only that folder, holding the session's
    # socket and a bundle trusting the session's own CA. The door answers
    # api.github.com, github.com, uploads.github.com and
    # codeload.github.com as GitHub would, adds the person's token, read
    # with gh and held in memory only, to the calls it allows, and says
    # each call and each refusal in the journal (github-door/src/lib.rs).
    # Started by the first fenced command that asks.
    systemd.user.sockets.pi-github-door = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "The door pi's fenced commands reach GitHub through";
      Socket = {
        ListenStream = "%t/pi-github/control.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
        Service = "pi-github-door.service";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
    systemd.user.services.pi-github-door = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "The door pi's fenced commands reach GitHub through";
        Requires = [ "pi-github-door.socket" ];
        After = [ "pi-github-door.socket" ];
      };
      Service.ExecStart = lib.escapeShellArgs [
        (lib.getExe githubDoor)
        "serve"
        "--sessions"
        "%t/pi-github/sessions"
        "--gh"
        (lib.getExe pkgs.gh)
      ];
    };

    # On the Claude plan every pi starts on it: the provider is one of the
    # extensions pi finds by itself, so the package list pi keeps is left
    # alone, and Sonnet is where it starts, a plan's limits being small.
    home.file.".pi/agent/extensions/claude-code" = lib.mkIf onClaude {
      source = "${import ./claude-provider.nix { inherit pkgs; }}/src";
    };
    home.sessionVariables = lib.mkMerge [
      (lib.mkIf onClaude { PI_CLAUDE_CODE_COMMAND = lib.getExe claude; })
      # A pi started by hand browses in Chromium, as a hosted one does
      # (host.nix says why). A default, so a machine's own value wins.
      (lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
        CHROME_PATH = lib.mkDefault (lib.getExe pkgs.chromium);
      })
    ];
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

    # The harness registers a codemode of its own, one that hands a long
    # script off after a minute, so pi leaves its built-in out and, unless
    # told to here, warns of it at every start. A package can't say it
    # replaces a built-in; only the settings can.
    jitsusama.pi.settings.extensions = lib.mkDefault [ "-builtin:codemode" ];

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

    # Every message typed during a turn reaches the agent at its next
    # boundary, rather than one per model call. Joel types in runs: on 13
    # November 57 of his messages queued behind a turn for 3,279 s in all,
    # and handing each run over together would have made it 1,668 s. Each
    # message, a peer's included, still arrives as its own.
    jitsusama.pi.settings.steeringMode = lib.mkDefault "all";

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
    # (chime.nix). It holds the ring while Do Not Disturb is on: `quiet`
    # names, under the runtime folder, the file the shell's Quiet.qml
    # writes, which a machine without the shell never has. Joel, 12
    # November: "we need to stop dinging when I am in do not disturb mode".
    # A Mac rings with its own sound, which the harness knows.
    xdg.configFile."pi/agentic-harness.pi/done-chime-widget/sound.json" =
      lib.mkIf pkgs.stdenv.hostPlatform.isLinux
        {
          text = builtins.toJSON {
            inherit (config.jitsusama.pi.chime) ding player;
            quiet = "quiet";
          };
        };

    home.file.".pi/agent/keybindings.json".text = builtins.toJSON (
      { "$schema" = "https://pi.dev/schemas/keybindings.schema.json"; } // keybindings
    );

    # The launcher's AI actions: the harness's server lists them and builds
    # each one's prompt, refusing a selection that looks like a secret, so
    # the launcher starts no program to read them or to use one. It's
    # TypeScript beside the harness's scrubber, which guards tool output
    # too, rather than a copy of it in Rust that would drift. Its socket is
    # held by systemd, so the launcher asking while it starts waits on it.
    # It's started by the first ask and ends itself after five minutes with
    # no connection open, so its 95 MB of node aren't kept all day for a
    # menu used now and then; a cold ask is answered in about 0.2 s.
    systemd.user.sockets.pi-verbs = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "Where the launcher asks for its AI actions";
        ConditionPathExists = verbsScript;
      };
      Socket = {
        ListenStream = "%t/daemons/pi-verbs.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
    systemd.user.services.pi-verbs = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "Lists the launcher's AI actions and builds their prompts";
        ConditionPathExists = verbsScript;
        Requires = [ "pi-verbs.socket" ];
        After = [ "pi-verbs.socket" ];
        X-SwitchMethod = "restart";
      };
      Service = {
        ExecStart = "@${lib.getExe piVerbs} pi-verbs serve";
        # A rest ends it cleanly, and the socket starts it again when asked.
        Restart = "on-failure";
        RestartSec = 5;
        Slice = "background.slice";
      };
    };

    jitsusama.keys.pi = map (key: {
      inherit (key) chord does;
      when = key.when or "";
    }) keys;
  };
}
