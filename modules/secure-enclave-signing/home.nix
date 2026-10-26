# Joel's signing keys in a Mac's Secure Enclave, as CryptoTokenKit
# identities made with `sc_auth create-ctk-identity -k p-256-ne`;
# ../signing says how a key is picked and what git is told. Neither key can
# be exported, backed up or moved, and the files under ~/.ssh are handles to
# them, not keys. Joel's asks for Touch ID on every signature; the agent's
# never asks, and signs while the screen is locked.
#
# OpenSSH reaches them through the Mac's own ssh-keychain.dylib, so there's
# no agent and no socket: the Mac's ssh-keygen signs with a handle once
# SSH_SK_PROVIDER names the library. se-keys (se-keys.sh) makes the
# identities and their handles, since a switch never should; a switch only
# says when a handle is missing.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama) identity;

  provider = "/usr/lib/ssh-keychain.dylib";
  handle = label: "${config.home.homeDirectory}/.ssh/${label}";

  # label, protection, name, email: "bio" asks for Touch ID on every
  # signature, "none" never does.
  identities = [
    [
      "joel-signing"
      "bio"
      identity.name
      (lib.defaultTo "" identity.email)
    ]
    [
      "agent-signing"
      "none"
      "${identity.name}'s agents"
      (lib.defaultTo "" identity.email)
    ]
  ];

  seKeys = pkgs.writeShellApplication {
    name = "se-keys";
    runtimeEnv = {
      inherit provider;
      # A file of one identity a line, its fields separated by tabs, as
      # se-keys.sh reads them. A file rather than the text itself, since a
      # name with an apostrophe, as the agents' key has, is quoted in the
      # script in a way shellcheck refuses.
      identities = pkgs.writeText "se-keys-identities" (
        lib.concatMapStringsSep "\n" (lib.concatStringsSep "\t") identities + "\n"
      );
    };
    text = builtins.readFile ./se-keys.sh;
  };
in
{
  imports = [ ../signing/home.nix ];

  jitsusama.signing.signer = {
    name = "se-sign";
    # The Mac's own, which ssh-keychain.dylib works with.
    sshKeygen = "/usr/bin/ssh-keygen";
    joelKey = handle "joel-signing";
    agentKey = handle "agent-signing";
    makesKeys = "se-keys";
    environment.SSH_SK_PROVIDER = "\${SSH_SK_PROVIDER:-${provider}}";
  };

  home.packages = [ seKeys ];

  home.activation.secureEnclaveHandles = lib.hm.dag.entryAfter [ "writeBoundary" ] (
    lib.concatMapStrings (
      label:
      let
        file = lib.escapeShellArg (handle label);
      in
      ''
        if [ ! -f ${file} ]; then
          warnEcho "secure-enclave-signing: "${file}" is missing; se-keys makes it."
        fi
      ''
    ) (map builtins.head identities)
  );
}
