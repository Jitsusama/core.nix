# glow, markdown read in the terminal, styled from the theme: headings in
# the strongest text, the rest quiet, and code in the terminal's colours as
# bat draws it.
{ config, pkgs, ... }:
let
  inherit (config.jitsusama.theme) roles terminal;

  # glamour's style format, the library glow draws with, as its own dark
  # style lays it out.
  style = {
    document = {
      block_prefix = "\n";
      block_suffix = "\n";
      color = roles.text;
      margin = 2;
    };
    block_quote = {
      indent = 1;
      indent_token = "│ ";
      color = roles.muted;
    };
    list.level_indent = 2;
    heading = {
      block_suffix = "\n";
      color = roles.strong;
      bold = true;
    };
    h1.prefix = "# ";
    h2.prefix = "## ";
    h3.prefix = "### ";
    h4.prefix = "#### ";
    h5.prefix = "##### ";
    h6.prefix = "###### ";
    strikethrough.crossed_out = true;
    emph.italic = true;
    strong = {
      bold = true;
      color = roles.strong;
    };
    hr = {
      color = roles.rule;
      format = "\n────────\n";
    };
    item.block_prefix = "• ";
    enumeration.block_prefix = ". ";
    task = {
      ticked = "[✓] ";
      unticked = "[ ] ";
    };
    link = {
      color = roles.muted;
      underline = true;
    };
    link_text.color = roles.strong;
    image = {
      color = roles.muted;
      underline = true;
    };
    image_text = {
      color = roles.muted;
      format = "Image: {{.text}}";
    };
    code = {
      prefix = " ";
      suffix = " ";
      color = roles.strong;
      background_color = roles.raised;
    };
    code_block = {
      color = roles.text;
      margin = 2;
      chroma = {
        text.color = roles.text;
        error.color = roles.alert;
        comment.color = roles.muted;
        comment_preproc.color = roles.muted;
        keyword.color = terminal.magenta;
        keyword_reserved.color = terminal.magenta;
        keyword_namespace.color = terminal.magenta;
        keyword_type.color = terminal.bright_yellow;
        operator.color = roles.muted;
        punctuation.color = roles.muted;
        name.color = roles.text;
        name_builtin.color = terminal.cyan;
        name_tag.color = terminal.magenta;
        name_attribute.color = roles.text;
        name_class.color = terminal.bright_yellow;
        name_constant.color = terminal.bright_red;
        name_decorator.color = terminal.cyan;
        name_exception.color = roles.alert;
        name_function.color = terminal.cyan;
        name_other.color = roles.text;
        literal.color = terminal.bright_red;
        literal_number.color = terminal.bright_red;
        literal_date.color = terminal.bright_red;
        literal_string.color = terminal.green;
        literal_string_escape.color = terminal.bright_cyan;
        generic_deleted.color = roles.alert;
        generic_emph.italic = true;
        generic_inserted.color = roles.success;
        generic_strong.bold = true;
        generic_subheading.color = roles.strong;
      };
    };
    definition_description.block_prefix = "\n→ ";
  };

  # glow's own settings, with the style it's drawn in added.
  settings = ''
    ${builtins.readFile ./glow.yml}
    style: ${pkgs.writeText "glow-style.json" (builtins.toJSON style)}
  '';

  # glow reads its settings from the Mac's preferences there and from the
  # XDG config directory elsewhere.
  file =
    if pkgs.stdenv.hostPlatform.isDarwin then
      "Library/Preferences/glow/glow.yml"
    else
      "${config.xdg.configHome}/glow/glow.yml";
in
{
  imports = [ ../theme/home.nix ];

  home.packages = [ pkgs.glow ];
  home.file.${file}.text = settings;
}
