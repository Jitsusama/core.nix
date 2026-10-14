# bat, cat with code coloured, and its helpers for diffs, man pages and
# watching a command, in a theme drawn from the machine's.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles terminal;

  # bat reads TextMate's theme format: the whole view's colours, then a
  # colour for each kind of code. Code takes the terminal's colours, which
  # each theme chooses for programs to draw with, and comments step back.
  scope = name: scopes: foreground: {
    inherit name;
    scope = lib.concatStringsSep ", " scopes;
    settings.foreground = foreground;
  };

  theme = {
    name = "Jitsusama";
    settings = [
      {
        settings = {
          inherit (roles) background;
          foreground = roles.text;
          caret = roles.strong;
          inherit (roles) selection;
          lineHighlight = roles.sunken;
          gutter = roles.background;
          gutterForeground = roles.muted;
        };
      }
      (scope "Comment" [ "comment" "punctuation.definition.comment" ] roles.muted)
      (scope "Keyword" [ "keyword" "storage" ] terminal.magenta)
      (scope "String" [ "string" "punctuation.definition.string" ] terminal.green)
      (scope "Escape" [ "constant.character.escape" "string.regexp" ] terminal.bright_cyan)
      (scope "Number" [ "constant.numeric" "constant.language" ] terminal.bright_red)
      (scope "Constant" [ "constant.other" "variable.other.constant" ] terminal.bright_red)
      (scope "Function" [ "entity.name.function" "support.function" ] terminal.cyan)
      (scope "Type" [
        "entity.name.type"
        "entity.name.class"
        "support.type"
        "support.class"
      ] terminal.bright_yellow)
      (scope "Tag" [ "entity.name.tag" ] terminal.magenta)
      # Nix names every binding an attribute, so they stay plain text.
      (scope "Attribute" [ "entity.other.attribute-name" "variable.parameter" ] roles.text)
      # A man page's options, what a reader scans it for.
      (scope "Option" [ "entity.name.command-line-option" ] terminal.cyan)
      (scope "Operator" [ "keyword.operator" "punctuation" ] roles.muted)
      (scope "Heading" [ "markup.heading" "entity.name.section" ] roles.strong)
      (scope "Link" [ "markup.underline.link" ] terminal.cyan)
      (scope "Inserted" [ "markup.inserted" ] roles.success)
      (scope "Deleted" [ "markup.deleted" ] roles.alert)
      (scope "Changed" [ "markup.changed" ] roles.warning)
      # Nothing is marked invalid: delta colours a diff's lines out of their
      # file, where a parser takes a stray brace for an error.
    ];
  };
in
{
  imports = [ ../theme/home.nix ];

  programs.bat = {
    enable = true;
    config = {
      style = "changes";
      theme = "Jitsusama";
    };
    themes.Jitsusama = {
      src = pkgs.writeTextDir "Jitsusama.tmTheme" (lib.generators.toPlist { escape = true; } theme);
      file = "Jitsusama.tmTheme";
    };
    extraPackages = [
      pkgs.bat-extras.batdiff
      pkgs.bat-extras.batman
      # batgrep has been broken since 2025-10-27.
      pkgs.bat-extras.batwatch
    ];
  };

  programs.zsh.shellAliases.cat = "bat";

  # Man pages read through bat too, as plain text it colours: col strips the
  # overstrikes man uses for bold, and MANROFFOPT keeps them from being
  # written as escapes that col can't strip.
  home.sessionVariables = {
    MANPAGER = "sh -c 'col -bx | ${lib.getExe config.programs.bat.package} --language=man --plain'";
    MANROFFOPT = "-c";
  };
}
