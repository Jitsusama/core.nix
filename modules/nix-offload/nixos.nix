# Nix's side of a builder lent to the machine while its machine queue has
# builds one could take: Nix's build hook reads the builder's line from
# /run/nix-offload/machines once, as each build command starts, and an
# empty list keeps that whole command here. The account's nix-offload
# (home.nix) writes it, and the queue reads it to count the builder's room.
# The file is empty while nothing is lent, so Nix never waits on a builder
# that isn't there. Standing builders are in machines-with-standing alone,
# which only a build that names it reads. The lent builder's store, which
# a build sent there as a command reaches it by, is named in
# /run/nix-offload/store, and the
# builder being made in /run/nix-offload/lending, so the queue holds the
# builds it could take until the line is there.
#
# Root reaches the builder with a key of its own, made once here and never
# held in a Nix value; where the builder is, and how root's ssh gets there,
# is the machine's to say.
#
# Writing the file chooses the builders root's Nix trusts, whose outputs
# come back unsigned. That's a trusted user's power already, since Nix lets
# one name builders for any build, so only a trusted account may write it:
# the file then gives it nothing it didn't have.
{
  config,
  pkgs,
  lib,
  ...
}:
{
  options.jitsusama.nix-offload.account = lib.mkOption {
    type = lib.types.str;
    description = "The account whose nix-offload lends the builder, and so writes its line.";
  };

  config = {
    assertions =
      let
        account = config.jitsusama.nix-offload.account;
        user = config.users.users.${account} or { };
        groups = [ (user.group or "") ] ++ (user.extraGroups or [ ]);
        trusted = config.nix.settings.trusted-users;
      in
      [
        {
          assertion = lib.elem account trusted || lib.any (group: lib.elem "@${group}" trusted) groups;
          message = ''
            jitsusama.nix-offload.account (${account}) writes the builders root's Nix
            trusts, so it must be one of nix.settings.trusted-users.
          '';
        }
      ];

    # NixOS gives Nix no builders unless builds are distributed. Its own
    # nix.buildMachines would go to /etc/nix/machines, which this file takes
    # the place of, so a machine's other builders belong in it too.
    nix.distributedBuilds = true;
    nix.settings = {
      builders = "@/run/nix-offload/machines";
      # The builder fetches what a cache holds itself, rather than being
      # sent it from here.
      builders-use-substitutes = true;
    };

    systemd.tmpfiles.rules = [
      "d /run/nix-offload 0755 ${config.jitsusama.nix-offload.account} - -"
      "f /run/nix-offload/machines 0644 ${config.jitsusama.nix-offload.account} - -"
      "f /run/nix-offload/machines-with-standing 0644 ${config.jitsusama.nix-offload.account} - -"
      "f /run/nix-offload/store 0644 ${config.jitsusama.nix-offload.account} - -"
      "f /run/nix-offload/lending 0644 ${config.jitsusama.nix-offload.account} - -"
    ];

    # After /etc, which /etc/nix belongs to.
    system.activationScripts.nix-offload-key = {
      deps = [ "etc" ];
      text = ''
        if [ ! -e /etc/nix/offload_ed25519 ]; then
          ${pkgs.openssh}/bin/ssh-keygen -q -t ed25519 -N "" -C nix-offload \
            -f /etc/nix/offload_ed25519
        fi
      '';
    };
  };
}
