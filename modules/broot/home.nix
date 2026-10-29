# broot, a tree to find a file in and act on it, skinned from the theme as
# the launcher is drawn: names in the text colour and directories in the
# strongest, the tree and what's beside the names quiet, the chosen row in
# the selection colour and the status line sunken below, with its keys
# brighter than its words.
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme) colours;
  mix = import ../theme/mix.nix { inherit lib; };
  rgb = import ../theme/rgb.nix { inherit lib; };
  # broot writes a style as its foreground, its background and then any
  # attributes, None leaving the terminal's own.
  style =
    foreground: background: attributes:
    lib.concatStringsSep " " (
      [
        (if foreground == null then "None" else rgb foreground)
        (if background == null then "None" else rgb background)
      ]
      ++ attributes
    );
  on = foreground: style foreground null [ ];
  status = foreground: style foreground colours.sunken [ ];
in
{
  imports = [ ../theme/home.nix ];

  programs.broot = {
    enable = true;
    enableZshIntegration = true;
    settings = {
      modal = false;
      show_selection_mark = false;
      terminal_title = "📁 {git-name}";
      icon_theme = "nerdfont";
      # Its verbs, without the skins its own configuration picks between.
      imports = [ "verbs.hjson" ];
      skin = {
        default = on colours.text;
        tree = on colours.rule;
        parent = on colours.muted;
        file = on colours.text;
        directory = style colours.strong null [ "Bold" ];
        exe = on colours.program;
        link = on colours.voice;
        pruning = style colours.muted null [ "Italic" ];
        perm__ = on colours.rule;
        perm_r = on colours.muted;
        perm_w = on colours.muted;
        perm_x = on colours.text;
        owner = on colours.muted;
        group = on colours.muted;
        count = on colours.muted;
        dates = on colours.muted;
        sparse = on colours.warning;
        content_extract = on colours.muted;
        content_match = style colours.strong colours.selection [ ];
        device_id_major = on colours.muted;
        device_id_sep = on colours.rule;
        device_id_minor = on colours.muted;
        git_branch = on colours.muted;
        git_insertions = on colours.success;
        git_deletions = on colours.alert;
        git_status_current = on colours.rule;
        git_status_modified = on colours.warning;
        git_status_staged = on colours.success;
        git_status_new = on colours.success;
        git_status_ignored = on colours.rule;
        git_status_conflicted = on colours.alert;
        git_status_other = on colours.alert;
        selected_line = style null colours.selection [ ];
        char_match = style colours.strong null [ "Bold" ];
        file_error = on colours.alert;
        flag_label = on colours.muted;
        flag_value = on colours.text;
        input = on colours.strong;
        status_error = style colours.on_accent colours.alert [ ];
        status_job = status colours.warning;
        status_normal = status colours.muted;
        status_italic = status colours.text;
        status_bold = status colours.strong;
        status_code = status colours.strong;
        status_ellipsis = status colours.muted;
        purpose_normal = on colours.muted;
        purpose_italic = on colours.text;
        purpose_bold = on colours.strong;
        purpose_ellipsis = on colours.muted;
        scrollbar_track = on colours.raised;
        scrollbar_thumb = on colours.muted;
        help_paragraph = on colours.text;
        help_bold = style colours.strong null [ "Bold" ];
        help_italic = on colours.strong;
        help_code = style colours.strong colours.raised [ ];
        help_headers = style colours.strong null [ "Bold" ];
        help_table_border = on colours.rule;
        preview = on colours.text;
        preview_title = status colours.strong;
        preview_line_number = on colours.muted;
        preview_separator = on colours.rule;
        preview_match = style colours.strong colours.selection [ ];
        diff_line_number = on colours.muted;
        diff_added = style colours.text (mix 0.25 colours.success colours.background) [ ];
        diff_removed = style colours.text (mix 0.18 colours.alert colours.background) [ ];
        hex_null = on colours.rule;
        hex_ascii_graphic = on colours.text;
        hex_ascii_whitespace = on colours.muted;
        hex_ascii_other = on colours.warning;
        hex_non_ascii = on colours.machine;
        staging_area_title = status colours.strong;
        # The one mark of a mode, in the accent, as a mode shows it's on.
        mode_command_mark = style colours.on_accent colours.accent [ "Bold" ];
      };
    };
  };
}
