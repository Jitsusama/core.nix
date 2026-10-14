# lsd, ls with icons and git's state, its long listing's columns quiet in
# the theme's text and muted, so the names, the newest files and what git
# has to say about them stand out.
{ config, ... }:
let
  inherit (config.jitsusama.theme) roles;
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
      user = roles.text;
      group = roles.muted;
      permission = {
        read = roles.muted;
        write = roles.muted;
        exec = roles.text;
        exec-sticky = roles.text;
        no-access = roles.rule;
        octal = roles.muted;
        acl = roles.muted;
        context = roles.muted;
      };
      date = {
        hour-old = roles.strong;
        day-old = roles.text;
        older = roles.muted;
      };
      size = {
        none = roles.muted;
        small = roles.muted;
        medium = roles.text;
        large = roles.strong;
      };
      inode = {
        valid = roles.muted;
        invalid = roles.alert;
      };
      links = {
        valid = roles.muted;
        invalid = roles.alert;
      };
      tree-edge = roles.rule;
      git-status = {
        default = roles.muted;
        unmodified = roles.muted;
        ignored = roles.rule;
        new-in-index = roles.success;
        new-in-workdir = roles.success;
        typechange = roles.warning;
        deleted = roles.alert;
        renamed = roles.warning;
        modified = roles.warning;
        conflicted = roles.alert;
      };
    };
  };
}
