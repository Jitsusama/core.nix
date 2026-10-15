# Joel's zsh prompt (prompt.zsh): where he is and where his keys go, then
# what's worth acting on, giving way a step at a time as the window
# narrows. The theme's roles and marks are written into its header here,
# so it starts nothing to find them, and git's state comes from git-engine
# through gitstatus's plugin.
# It's kept as a file in the account as well, which is what kitty's ssh
# kitten carries to another machine.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles glyphs;
  cfg = config.jitsusama.prompt;

  quote = lib.escapeShellArg;
  table =
    name: attrs:
    lib.concatLines (
      [ "typeset -gA ${name}=(" ]
      ++ lib.mapAttrsToList (key: value: "  ${quote key} ${quote value}") attrs
      ++ [ ")" ]
    );

  header = lib.concatLines [
    "# Written from jitsusama.theme and jitsusama.prompt."
    (table "_prompt_hue" roles)
    (table "_prompt_glyph" (lib.mapAttrs (_: mark: mark.glyph) glyphs))
    (table "_prompt_console" (lib.mapAttrs (_: mark: mark.console) glyphs))
    "typeset -ga _prompt_roots=(${lib.concatMapStringsSep " " quote cfg.roots})"
    "typeset -ga _prompt_steps=(${lib.concatStringsSep " " cfg.givesWay})"
    "typeset -gi _prompt_long=${toString cfg.longRun}"
    "typeset -g _prompt_gitstatus=${gitstatus}/gitstatus.plugin.zsh"
    "typeset -g GITSTATUS_DAEMON=${lib.getExe engine}"
    "typeset -g GITSTATUS_AUTO_INSTALL=0"
  ];

  prompt = pkgs.writeText "prompt.zsh" (header + builtins.readFile ./prompt.zsh);

  # Answers what the prompt asks of a repository as gitstatusd would, but
  # reads World, whose refs and index gitstatusd's libgit2 can't.
  engine = pkgs.callPackage ../git-engine/package.nix { };

  # gitstatus's plugin, starting no program it needn't: the machine it runs
  # on is known when it's built rather than asked of uname, and the watchdog
  # beside the daemon waits in zsh rather than in sleep. nixpkgs' install
  # script names gitstatusd over whatever the caller asked for, so it names
  # git-engine instead.
  machine =
    let
      inherit (pkgs.stdenv.hostPlatform.uname) system processor;
    in
    "${lib.toLower system} ${processor}";
  asked = "\"\${\${(L)$(command uname -sm)}//ı/i}\"";
  gitstatus = pkgs.runCommandLocal "gitstatus-plugin" { } ''
    cp -r ${pkgs.gitstatus}/share/gitstatus $out
    chmod -R u+w $out
    substituteInPlace $out/gitstatus.plugin.zsh \
      --replace-fail ${lib.escapeShellArg asked} ${lib.escapeShellArg "\"${machine}\""} \
      --replace-fail 'command sleep 5' '{ zselect -t 500 || : }'
    substituteInPlace $out/install \
      --replace-fail ${lib.escapeShellArg "GITSTATUS_DAEMON=${lib.getExe' pkgs.gitstatus "gitstatusd"}"} \
        ${lib.escapeShellArg "GITSTATUS_DAEMON=${lib.getExe engine}"}
  '';
in
{
  imports = [
    ../theme/home.nix
    ../zsh/home.nix
  ];

  options.jitsusama.prompt = {
    file = lib.mkOption {
      type = lib.types.path;
      readOnly = true;
      default = prompt;
      defaultText = lib.literalMD "prompt.zsh with its header";
      description = "The prompt, as zsh sources it.";
    };

    roots = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ".git" ];
      description = ''
        Files or folders that mark a root, which the prompt tells where it is
        from: the nearest folder holding one names it. A machine whose
        repositories hold smaller roots names their marker before `.git`.
      '';
    };

    givesWay = lib.mkOption {
      type = lib.types.listOf (
        lib.types.enum [
          "time"
          "shell"
          "branchshort"
          "pathshort"
          "branchglyph"
          "right"
          "opshort"
        ]
      );
      default = [
        "time"
        "shell"
        "branchshort"
        "pathshort"
        "branchglyph"
        "right"
        "opshort"
      ];
      description = ''
        What gives way when the window is narrow, the first to go first: a
        long command's time, the development shell and jobs, the branch's
        middle, the path's middle, the branch to its glyph, the whole right
        side, and what's under way to its glyph. A step left out never
        happens.
      '';
    };

    longRun = lib.mkOption {
      type = lib.types.ints.positive;
      default = 5;
      description = "How many seconds a command runs before its prompt says how long.";
    };
  };

  config = {
    xdg.dataFile."jitsusama/prompt.zsh".source = prompt;

    # Early, before completion, so git-engine starts while the rest of the
    # shell's setup runs. Each precmd hook is handed the last command's status
    # afresh, so the prompt still sees a failure wherever it sits.
    programs.zsh.initContent = lib.mkOrder 500 "source ${prompt}";
  };
}
