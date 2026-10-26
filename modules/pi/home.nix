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
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme)
    appearance
    glyphs
    roles
    syntax
    ;
  mix = import ../theme/mix.nix { inherit lib; };

  # A tool's result is a faint tint of how it went, under text that stays
  # the text's own colour.
  tint = weight: colour: mix weight colour roles.background;

  # pi's own names for what it colours, in its theme format:
  # https://pi.dev/schemas/theme.schema.json
  theme = {
    name = "jitsusama";
    inherit appearance;
    colors = {
      inherit (roles)
        accent
        text
        success
        warning
        ;
      error = roles.alert;
      border = roles.rule;
      borderAccent = roles.accent;
      borderMuted = roles.edge;
      inherit (roles) muted;
      dim = roles.muted;
      thinkingText = roles.muted;
      selectedBg = roles.selection;
      searchMatchBg = roles.raised;
      searchMatchText = roles.strong;
      scrollbarTrack = roles.raised;
      scrollbarThumb = roles.rule;
      # What Joel wrote sits on the sunken shade, a step below the rest.
      userMessageBg = roles.sunken;
      userMessageText = roles.text;
      customMessageBg = roles.sunken;
      customMessageText = roles.muted;
      customMessageLabel = roles.voice;
      toolPendingBg = roles.sunken;
      toolSuccessBg = tint 0.1 roles.success;
      toolErrorBg = tint 0.12 roles.alert;
      toolTitle = roles.strong;
      toolOutput = roles.muted;
      # Markdown reads as Neovim draws it: headings strong, code as strings,
      # the marks between muted.
      mdHeading = roles.strong;
      mdLink = roles.program;
      mdLinkUrl = roles.muted;
      mdCode = syntax.string.colour;
      mdCodeBlock = roles.text;
      mdCodeBlockBorder = roles.rule;
      mdQuote = roles.muted;
      mdQuoteBorder = roles.rule;
      mdHr = roles.rule;
      mdListBullet = syntax.punctuation.colour;
      toolDiffAdded = roles.success;
      toolDiffRemoved = roles.alert;
      toolDiffContext = roles.muted;
      syntaxComment = syntax.comment.colour;
      syntaxKeyword = syntax.keyword.colour;
      syntaxFunction = syntax.function.colour;
      syntaxVariable = roles.text;
      syntaxString = syntax.string.colour;
      syntaxNumber = syntax.constant.colour;
      syntaxType = syntax.type.colour;
      syntaxOperator = syntax.operator.colour;
      syntaxPunctuation = syntax.punctuation.colour;
      # How hard pi thinks warms from the muted text to the alert, a step
      # through the glyphs' hues at each level.
      thinkingOff = roles.muted;
      thinkingMinimal = syntax.type.colour;
      thinkingLow = roles.program;
      thinkingMedium = roles.voice;
      thinkingHigh = roles.person;
      thinkingXhigh = roles.warning;
      thinkingMax = roles.alert;
      # A shell command is something to do.
      bashMode = roles.action;
    };
    export = {
      pageBg = roles.background;
      cardBg = roles.sunken;
      infoBg = roles.raised;
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

  # On the Claude plan, the machine's own Claude Code login is the plan's.
  piClaude = import ./claude.nix {
    inherit pkgs claude;
    ownLogin = onClaude;
  };

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
    ../theme/home.nix
    ../keys/home.nix
    ../prompt/home.nix
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
  };

  config = {
    home.file.".pi/agent/themes/jitsusama.json".text = builtins.toJSON theme;

    home.packages = lib.optional (!onClaude) piClaude ++ [
      (import ./ask.nix {
        inherit pkgs lib piClaude;
        inherit (config.jitsusama.pi) ask;
      })
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
    jitsusama.pi.settings.defaultProvider = lib.mkIf onClaude "claude-code";
    jitsusama.pi.settings.defaultModel = lib.mkIf onClaude "claude-sonnet-5";

    # The room kept for what a compaction writes. pi's default, 16,384
    # tokens, runs out summarising a long session, which then compacts
    # nothing and runs on toward the model's limit.
    jitsusama.pi.settings.compaction.reserveTokens = lib.mkDefault 64000;

    home.activation.piTakesCoresSettings = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      run ${
        lib.getExe (
          import ./settings.nix {
            inherit pkgs;
            inherit (config.jitsusama.pi) settings;
          }
        )
      } ${lib.escapeShellArg "${config.home.homeDirectory}/.pi/agent/settings.json"}
    '';

    # Where the harness's status line learns the folder and git; its README
    # says what each field means.
    xdg.configFile."pi/agentic-harness.pi/status-line/where.json".text = builtins.toJSON {
      engine = lib.getExe (pkgs.callPackage ../git-engine/package.nix { });
      inherit (config.jitsusama.prompt) roots;
      marks = lib.mapAttrs (_: mark: mark.glyph) {
        inherit (glyphs)
          branch
          unpushed
          more
          complete
          active
          pending
          thinking
          ;
        unsaved = glyphs.on;
      };
    };

    home.file.".pi/agent/keybindings.json".text = builtins.toJSON (
      { "$schema" = "https://pi.dev/schemas/keybindings.schema.json"; } // keybindings
    );

    jitsusama.keys.pi = map (key: {
      inherit (key) chord does;
      when = key.when or "";
    }) keys;
  };
}
