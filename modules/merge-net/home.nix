# The net under core's main: a daemon that runs the VM tests each merge
# changed once it lands, and reverts a merge that breaks one
# (tools/merge-net). A merge lands on its own checks, VM tests aside, and
# the net catches what they would have. It keeps its state, its logs and
# its own checkout in ~/.local/state/net, so it carries on across restarts
# and reboots, and it starts nothing while the machine is held.
#
# It runs git, gh, nix and nix-offload as the account does, with the
# account's settings, so its commits are signed as the account's are and
# its PRs are the account's; the machine gives nix and nix-offload.
#
# Beside it, `changed-checks keep-main` keeps each head of main's checks
# where nix-offload sends changed-checks runs, once the head has stood two
# minutes, so a branch cut from it pays for its own tree alone rather than
# the first run after each merge paying for the base too. A builder lent
# afresh gets it again; with none lent it sends nothing, so it never asks
# for one or holds one lent. It keeps its clone in
# ~/.local/state/changed-checks/main, and evaluates nothing here.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  net = pkgs.callPackage ../../tools/merge-net/package.nix { };
  changed-checks = pkgs.callPackage ../../tools/changed-checks/package.nix { };
  # The account's own tools come first, the machine's next, and these last,
  # for a machine without them.
  path = lib.concatStringsSep ":" [
    "/etc/profiles/per-user/${config.home.username}/bin"
    "${config.home.homeDirectory}/.nix-profile/bin"
    "/run/wrappers/bin"
    "/run/current-system/sw/bin"
    (lib.makeBinPath [
      changed-checks
      pkgs.gh
      pkgs.git
    ])
  ];
in
{
  home.packages = [ net ];

  systemd.user.services.merge-net = {
    Unit.Description = "Runs the VM tests each merge to core's main changed, and reverts a merge that breaks one";
    Service = {
      ExecStart = "${lib.getExe net} watch";
      Environment = [ "PATH=${path}" ];
      Restart = "always";
      RestartSec = 60;
    };
    Install.WantedBy = [ "default.target" ];
  };

  systemd.user.services.changed-checks-keeps-main = {
    Unit.Description = "Keeps each head of core's main's checks where changed-checks runs are sent";
    Service = {
      ExecStart = "${lib.getExe changed-checks} keep-main";
      Environment = [ "PATH=${path}" ];
      Restart = "always";
      RestartSec = 60;
    };
    Install.WantedBy = [ "default.target" ];
  };
}
