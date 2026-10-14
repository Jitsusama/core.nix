# broot, a tree to find a file in and act on it, skinned from the theme as
# the launcher is drawn: names in the text colour and directories in the
# strongest, the tree and what's beside the names quiet, the chosen row in
# the selection colour and the status line sunken below, with its keys
# brighter than its words.
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme) roles;
  mix = import ../theme/mix.nix { inherit lib; };
  channel = colour: index: toString (lib.fromHexString (builtins.substring (1 + index * 2) 2 colour));
  rgb = colour: "rgb(${channel colour 0}, ${channel colour 1}, ${channel colour 2})";
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
  status = foreground: style foreground roles.sunken [ ];
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
        default = on roles.text;
        tree = on roles.rule;
        parent = on roles.muted;
        file = on roles.text;
        directory = style roles.strong null [ "Bold" ];
        exe = on roles.program;
        link = on roles.voice;
        pruning = style roles.muted null [ "Italic" ];
        perm__ = on roles.rule;
        perm_r = on roles.muted;
        perm_w = on roles.muted;
        perm_x = on roles.text;
        owner = on roles.muted;
        group = on roles.muted;
        count = on roles.muted;
        dates = on roles.muted;
        sparse = on roles.warning;
        content_extract = on roles.muted;
        content_match = style roles.strong roles.selection [ ];
        device_id_major = on roles.muted;
        device_id_sep = on roles.rule;
        device_id_minor = on roles.muted;
        git_branch = on roles.muted;
        git_insertions = on roles.success;
        git_deletions = on roles.alert;
        git_status_current = on roles.rule;
        git_status_modified = on roles.warning;
        git_status_staged = on roles.success;
        git_status_new = on roles.success;
        git_status_ignored = on roles.rule;
        git_status_conflicted = on roles.alert;
        git_status_other = on roles.alert;
        selected_line = style null roles.selection [ ];
        char_match = style roles.strong null [ "Bold" ];
        file_error = on roles.alert;
        flag_label = on roles.muted;
        flag_value = on roles.text;
        input = on roles.strong;
        status_error = style roles.on_accent roles.alert [ ];
        status_job = status roles.warning;
        status_normal = status roles.muted;
        status_italic = status roles.text;
        status_bold = status roles.strong;
        status_code = status roles.strong;
        status_ellipsis = status roles.muted;
        purpose_normal = on roles.muted;
        purpose_italic = on roles.text;
        purpose_bold = on roles.strong;
        purpose_ellipsis = on roles.muted;
        scrollbar_track = on roles.raised;
        scrollbar_thumb = on roles.muted;
        help_paragraph = on roles.text;
        help_bold = style roles.strong null [ "Bold" ];
        help_italic = on roles.strong;
        help_code = style roles.strong roles.raised [ ];
        help_headers = style roles.strong null [ "Bold" ];
        help_table_border = on roles.rule;
        preview = on roles.text;
        preview_title = status roles.strong;
        preview_line_number = on roles.muted;
        preview_separator = on roles.rule;
        preview_match = style roles.strong roles.selection [ ];
        diff_line_number = on roles.muted;
        diff_added = style roles.text (mix 0.25 roles.success roles.background) [ ];
        diff_removed = style roles.text (mix 0.18 roles.alert roles.background) [ ];
        hex_null = on roles.rule;
        hex_ascii_graphic = on roles.text;
        hex_ascii_whitespace = on roles.muted;
        hex_ascii_other = on roles.warning;
        hex_non_ascii = on roles.machine;
        staging_area_title = status roles.strong;
        # The one mark of a mode, in the accent, as a mode shows it's on.
        mode_command_mark = style roles.on_accent roles.accent [ "Bold" ];
      };
    };
  };
}
