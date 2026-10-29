# Nix's side of a builder lent to the machine while its machine queue holds
# builds back: nix-daemon reads the builder's line from
# /run/nix-offload/machines at each build, the account's nix-offload
# (home.nix) writes it, and the queue reads it to count the builder's room.
# The file is empty while nothing is lent, so Nix never waits on a builder
# that isn't there.
#
# Root reaches the builder with a key of its own, made once here and never
# held in a Nix value; where the builder is, and how root's ssh gets there,
# is the machine's to say.
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
    ];

    system.activationScripts.nix-offload-key = ''
      if [ ! -e /etc/nix/offload_ed25519 ]; then
        ${pkgs.openssh}/bin/ssh-keygen -q -t ed25519 -N "" -C nix-offload \
          -f /etc/nix/offload_ed25519
      fi
    '';
  };
}
