# The link keys of ../link-keys on a Linux machine with no TPM, such as an
# Apple silicon Mac: each realm's key a plain file on the encrypted disk,
# served at <realm>-link.sock by an agent holding only it, which asks
# nothing (../key-file-agent), so mycelium's node reaches it as it would a
# key in the TPM. Anything running as the account could copy it, as it
# could the agents' key beside it.
#
# Like the Mac's signing keys, a key is made by hand, once:
# `ssh-keygen -t ed25519 -N '' -C "$USER@$(hostname) <realm> link" -f
# ~/.local/share/links/<realm>/<realm>-link`, and a switch only says when
# one is missing.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.linkKeys;
  file = realm: "${config.xdg.dataHome}/links/${realm}/${realm}-link";
  served = map (
    realm: import ../key-file-agent/units.nix { inherit lib pkgs; } "${realm}-link" (file realm)
  ) cfg;
in
{
  options.jitsusama.linkKeys = lib.mkOption {
    type = lib.types.listOf (lib.types.strMatching "[a-z][a-z0-9-]*");
    default = [ ];
    example = [ "personal" ];
    description = ''
      The realms whose node signs in to other machines with a key of its
      own, each served on <realm>-link.sock in the runtime folder by an agent
      holding only that key.
    '';
  };

  config = {
    systemd.user = {
      sockets = lib.mkMerge (map (units: units.sockets) served);
      services = lib.mkMerge (map (units: units.services) served);
      # Each key's folder, there and private before its key is made.
      tmpfiles.rules = map (realm: "d ${dirOf (file realm)} 0700 - - -") cfg;
    };

    home.activation.fileLinkKeys = lib.hm.dag.entryAfter [ "writeBoundary" ] (
      lib.concatMapStrings (
        realm:
        let
          quoted = lib.escapeShellArg (file realm);
        in
        ''
          if [ ! -f ${quoted} ]; then
            warnEcho "file-link-keys: "${quoted}" is missing; the ssh-keygen in modules/file-link-keys/home.nix makes it."
          fi
        ''
      ) cfg
    );
  };
}
