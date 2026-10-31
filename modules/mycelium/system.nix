# The other end of mycelium's links, on a machine's system: the link keys of
# the machines that link to it, each pinned in its account's authorized keys
# to `mycelium ssh --from NAME`, so a key runs only its machine's end of a
# link or an attach of a session held here, as that machine, and never a
# shell or any other command. A machine names each by realm and by the
# linking machine's name, as its links know it:
#
#   jitsusama.mycelium = {
#     account = "someone";
#     linkedFrom.personal.other-machine = "ecdsa-sha2-nistp256 AAAA...";
#   };
#
# Its own keys, a person's, are the machine's to authorize as they are, and
# a key named here should be no other key, since it is pinned (link-keys
# makes each realm's). NixOS and nix-darwin both keep authorized keys so.
{ mycelium }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.mycelium;
  package = mycelium.packages.${pkgs.stdenv.hostPlatform.system}.default;
  name = lib.types.strMatching "[a-z0-9-]+";
  # An attach joins a session held here, so the key may have a terminal.
  pinned =
    realm: machine: key:
    ''restrict,pty,command="${lib.getExe package} --realm ${realm} ssh --from ${machine}" ${key}'';
in
{
  options.jitsusama.mycelium = {
    account = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "someone";
      description = "The account whose node the machines in `linkedFrom` link to.";
    };

    linkedFrom = lib.mkOption {
      type = lib.types.attrsOf (lib.types.attrsOf lib.types.str);
      default = { };
      example = {
        personal.other-machine = "ecdsa-sha2-nistp256 AAAA...";
      };
      description = ''
        The link key of each machine that links to this one, by realm and
        by that machine's name, each pinned to its end of a link or an
        attach in that realm, as that machine.
      '';
    };
  };

  config = {
    assertions = [
      {
        assertion = cfg.linkedFrom == { } || cfg.account != null;
        message = "jitsusama.mycelium.linkedFrom needs the account they link to, in jitsusama.mycelium.account.";
      }
      # Both go into the key's command, so each is a plain name.
      {
        assertion = lib.all name.check (
          lib.concatLists (
            lib.mapAttrsToList (realm: machines: [ realm ] ++ lib.attrNames machines) cfg.linkedFrom
          )
        );
        message = "Each realm and machine in jitsusama.mycelium.linkedFrom is a name of a-z, 0-9 and -.";
      }
    ];

    users.users = lib.mkIf (cfg.linkedFrom != { } && cfg.account != null) {
      ${cfg.account}.openssh.authorizedKeys.keys = lib.concatLists (
        lib.mapAttrsToList (realm: lib.mapAttrsToList (pinned realm)) cfg.linkedFrom
      );
    };
  };
}
