# The checks of pi, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  holds,
  lib,
  macWorkstation,
  pkgs,
  workstation,
  ...
}:
{
  # The harness's weekly measure runs each Monday in the background,
  # catching up on a Monday slept through, with the store's node, and only
  # where the harness is installed; a Mac, with no systemd, gets none.
  pi-measures-each-week-in-the-background =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-weekly-measure;
      timer = home.systemd.user.timers.pi-weekly-measure.Timer;
      # home-manager keeps a unit's settings as lists.
      first = value: lib.head (lib.toList value);
    in
    holds "pi-measures-each-week-in-the-background"
      "pi's weekly measure runs outside background.slice, not on Mondays, misses a Monday slept through, runs a node off the PATH, fails where the harness isn't installed, or reaches a Mac."
      (
        service.Service.Slice == "background.slice"
        && timer.Persistent
        && lib.hasPrefix "Mon " (first timer.OnCalendar)
        && lib.hasPrefix "${lib.getExe pkgs.nodejs} " (first service.Service.ExecStart)
        && lib.hasSuffix "/scripts/weekly-measure.ts" (first service.Unit.ConditionPathExists)
        && !(mac.systemd.user.services ? pi-weekly-measure)
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
      engine = lib.getExe (pkgs.callPackage ../git-engine/package.nix { });
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
      ${lib.getExe pkgs.bash} ${../../tests/pi-keeps-its-own-settings.sh} ${program} ${machines}
      touch $out
    '';

  # pi-claude starts pi on the plan alone: the provider, Sonnet unless told
  # otherwise, off the mesh, and nothing that would bill an API instead; and
  # an account gets it.
  pi-claude-keeps-to-the-plan =
    let
      home = account workstation;
      # Claude Code as pi-claude starts it, writing down how.
      claude = pkgs.writeShellScriptBin "claude" ''
        printf 'arg %s\n' "$@" >> "$SAID"
        env | sed 's/^/env /' >> "$SAID"
      '';
      program = lib.getExe (import ./claude.nix { inherit pkgs claude; });
      onOwnLogin = lib.getExe (
        import ./claude.nix {
          inherit pkgs claude;
          ownLogin = true;
        }
      );
    in
    assert lib.assertMsg (lib.any (
      package: lib.getName package == "pi-claude"
    ) home.home.packages) "An account with pi doesn't get pi-claude.";
    pkgs.runCommandLocal "pi-claude-keeps-to-the-plan" { } ''
      ${lib.getExe pkgs.bash} ${../../tests/pi-claude-keeps-to-the-plan.sh} ${program} \
        ${lib.getExe claude} ${onOwnLogin}
      touch $out
    '';

  # A used-up Claude plan limit is said plainly, by the provider as it is
  # patched: which limit, when it resets, and that nothing went to extra
  # usage, in words pi never retries.
  pi-says-when-the-plan-runs-out = pkgs.runCommandLocal "pi-says-when-the-plan-runs-out" { } ''
    ${lib.getExe pkgs.nodejs} --experimental-strip-types --no-warnings \
      ${../../tests/pi-says-when-the-plan-runs-out.mjs} ${
        import ./claude-provider.nix { inherit pkgs; }
      }
    touch $out
  '';

  # A machine on the Claude plan has one pi, on it: the provider is an
  # extension pi finds by itself, pi starts on it and on Sonnet, Claude Code
  # is the one it drives, and there is no pi-claude beside it. The Ask row
  # follows, on the machine's own login. A work machine has none of this.
  pi-runs-on-the-machines-plan =
    let
      onClaude = account (
        workstation.extendModules {
          modules = [ { home-manager.users.someone.jitsusama.pi.plan = "claude"; } ];
        }
      );
      onPi = account workstation;
      names = home: map lib.getName home.home.packages;
      provider = onClaude.home.file.".pi/agent/extensions/claude-code".source;
    in
    holds "pi-runs-on-the-machines-plan"
      "A machine's pi isn't on the plan it names, or a work machine's pi changed."
      (
        builtins.pathExists "${provider}/index.ts"
        && onClaude.jitsusama.pi.settings.defaultProvider == "claude-code"
        && onClaude.jitsusama.pi.settings.defaultModel == "claude-sonnet-5"
        && onClaude.home.sessionVariables ? PI_CLAUDE_CODE_COMMAND
        && !(lib.elem "pi-claude" (names onClaude))
        && onClaude.jitsusama.pi.ask.plan == "claude"
        && !(onPi.home.file ? ".pi/agent/extensions/claude-code")
        && !(onPi.jitsusama.pi.settings ? defaultProvider)
        && lib.elem "pi-claude" (names onPi)
        && onPi.jitsusama.pi.ask.plan == "pi"
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
    in
    holds "pi-loads-its-packages"
      "pi's package list isn't core's two, or a machine's package replaced them."
      (
        plain.jitsusama.pi.settings.packages == core
        && lib.take 2 added.jitsusama.pi.settings.packages == core
        && lib.elem own added.jitsusama.pi.settings.packages
      );

  # pi-ask starts pi for the Ask row with nothing loaded but the machine's
  # extensions, on the machine's plan and model: pi's own by default, with
  # its quick model, or pi-claude's; and an account gets it.
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
        }
      );
      onClaude = asking {
        plan = "claude";
        model = "claude-code/claude-haiku-4-5-20251001";
        extensions = [ ];
      };
    in
    assert lib.assertMsg (lib.any (
      package: lib.getName package == "pi-ask"
    ) home.home.packages) "An account with pi doesn't get pi-ask.";
    assert lib.assertMsg (
      home.jitsusama.pi.ask.plan == "pi"
    ) "pi-ask's plan isn't pi's own by default.";
    pkgs.runCommandLocal "pi-ask-answers-on-the-machines-plan" { } ''
      ${lib.getExe pkgs.bash} ${../../tests/pi-ask-answers-on-the-machines-plan.sh} \
        ${onPi} ${onClaude} ${lib.getExe piClaude}
      touch $out
    '';
}
