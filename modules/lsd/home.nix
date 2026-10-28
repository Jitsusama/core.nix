# lsd, ls with icons and git's state, its long listing's columns quiet in
# the theme's text and muted, so the names, the newest files and what git
# has to say about them stand out.
{ config, ... }:
let
  inherit (config.jitsusama.theme) colours;
in
{
  imports = [ ../theme/home.nix ];

  programs.lsd = {
    enable = true;
    settings = {
      blocks = [
        "permission"
        "user"
        "group"
        "size"
        "date"
        "git"
        "name"
      ];
      date = "+%Y-%m-%dT%H:%M:%S";
    };
    colors = {
      user = colours.text;
      group = colours.muted;
      permission = {
        read = colours.muted;
        write = colours.muted;
        exec = colours.text;
        exec-sticky = colours.text;
        no-access = colours.rule;
        octal = colours.muted;
        acl = colours.muted;
        context = colours.muted;
      };
      date = {
        hour-old = colours.strong;
        day-old = colours.text;
        older = colours.muted;
      };
      size = {
        none = colours.muted;
        small = colours.muted;
        medium = colours.text;
        large = colours.strong;
      };
      inode = {
        valid = colours.muted;
        invalid = colours.alert;
      };
      links = {
        valid = colours.muted;
        invalid = colours.alert;
      };
      tree-edge = colours.rule;
      git-status = {
        default = colours.muted;
        unmodified = colours.muted;
        ignored = colours.rule;
        new-in-index = colours.success;
        new-in-workdir = colours.success;
        typechange = colours.warning;
        deleted = colours.alert;
        renamed = colours.warning;
        modified = colours.warning;
        conflicted = colours.alert;
      };
    };
  };
}
