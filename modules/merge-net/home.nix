# The net under core's main: a daemon that runs the checks each merge
# changed once it lands, VM tests and the rest, and reverts a merge that
# breaks one (tools/merge-net). A merge lands on its own checks, VM tests
# aside, and the net catches what they would have, and what a branch built
# before its rebase missed. It keeps its state, its logs and
# its own checkout in ~/.local/state/net, so it carries on across restarts
# and reboots, and it starts nothing while the machine is held.
#
# It runs git, gh, nix and nix-offload as the account does, with the
# account's settings, so its commits are signed as the account's are and
# its PRs are the account's; the machine gives nix and nix-offload.
#
# Beside it, `changed-checks keep-main` keeps each head of main's checks
# where nix-offload sends changed-checks runs, once the head has stood 30
# seconds, so a branch cut from it pays for its own tree alone rather than
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
  installed = "changed-checks/installed";
  installedNet = "${config.home.profileDirectory}/bin/merge-net";
  installedKeeper = "${config.xdg.stateHome}/${installed}/bin/changed-checks";
  # The account's own tools come first, the machine's next, and these last,
  # for a machine without them.
  path = lib.concatStringsSep ":" [
    "/etc/profiles/per-user/${config.home.username}/bin"
    "${config.home.homeDirectory}/.nix-profile/bin"
    "/run/wrappers/bin"
    "/run/current-system/sw/bin"
    (lib.makeBinPath [
      changed-checks
      # The train gives each call to gh a time limit through timeout.
      pkgs.coreutils
      pkgs.gh
      pkgs.git
    ])
  ];
in
{
  home.packages = [ net ];

  # A switch leaves the running net be, since stopping it mid-round loses
  # every run in flight; it follows the build installed and, once that is
  # another, starts no more runs and ends when those under way have, for
  # Restart to start the new one. It starts the build it follows, not its
  # own store path: on 20 November a switch moved the profile and rolled
  # back before the unit was reloaded, and from 19:04 to 19:13 each
  # restart ran the old build again, which saw itself replaced and left.
  systemd.user.services.merge-net = {
    Unit = {
      Description = "Runs the VM tests each merge to core's main changed, and reverts a merge that breaks one";
      X-SwitchMethod = "keep-old";
    };
    Service = {
      ExecStart = "${installedNet} watch --follow ${installedNet}";
      Environment = [ "PATH=${path}" ];
      Restart = "always";
      RestartSec = 60;
      # Below what Joel uses, as the timed work it is: its evaluations ran
      # in app.slice beside his programs, 2.2 GiB at their peak on
      # 10 November.
      Slice = "jobs-timed.slice";
    };
    Install.WantedBy = [ "default.target" ];
  };

  # The keeper follows the build linked here, as the net follows its own:
  # a switch leaves it be, and it ends between keeps once this names
  # another, rather than losing the keep under way. Not in the profile,
  # where it would stand on every PATH beside a checkout's own.
  xdg.stateFile.${installed}.source = changed-checks;

  systemd.user.services.changed-checks-keeps-main = {
    Unit = {
      Description = "Keeps each head of core's main's checks where changed-checks runs are sent";
      X-SwitchMethod = "keep-old";
    };
    Service = {
      # The build it follows, for the reason the net starts its own.
      ExecStart = "${installedKeeper} keep-main --follow ${installedKeeper}";
      Environment = [ "PATH=${path}" ];
      Restart = "always";
      RestartSec = 60;
      Slice = "jobs-timed.slice";
    };
    Install.WantedBy = [ "default.target" ];
  };
}
