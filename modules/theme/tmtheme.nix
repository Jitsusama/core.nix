# The theme as TextMate's theme format, which bat and inlyne read through
# syntect to colour code: the whole view's colours, then a colour for each
# kind of code. Code takes the terminal's colours, which each theme chooses
# for programs to draw with, and comments step back. Each program says what
# the code sits on: bat the terminal's background, inlyne a block a step
# below its page.
#
#   import ./tmtheme.nix { inherit lib pkgs roles terminal; } roles.background
#     →  a folder holding Jitsusama.tmTheme
{
  lib,
  pkgs,
  roles,
  terminal,
}:
background:
let
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
          inherit background;
          foreground = roles.text;
          caret = roles.strong;
          inherit (roles) selection;
          lineHighlight = roles.sunken;
          gutter = background;
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
pkgs.writeTextDir "Jitsusama.tmTheme" (lib.generators.toPlist { escape = true; } theme)
