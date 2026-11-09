# Joel's signing keys on a Linux machine with no TPM, such as an Apple
# silicon Mac; ../signing says how a key is picked and what git is told.
#
# Joel's key is a resident ed25519-sk key on his YubiKey, which can't be
# copied off it and asks for its PIN and a touch on every signature
# (verify-required). ssh-keygen signs with its handle, a file that's useless
# without the YubiKey, so it needs no agent. The agents' key is a plain file
# on the encrypted disk, which asks nothing, so it signs with nobody at the
# keyboard; anything running as Joel could copy it, as it could the agents'
# own tokens beside it. It's served on agent-signing.sock too, as
# tpm-signing serves its own, so mycelium's node signs its links in with it.
#
# The guide makes account keys only in a TPM, so yubikey-keys
# (yubikey-keys.sh) makes these, once, after the install, as the Mac's
# se-keys does; a switch only says when one is missing.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  joelKey = "${config.xdg.dataHome}/signing/joel-signing";
  agentKey = "${config.home.homeDirectory}/.ssh/agent-signing";
  served = import ../key-file-agent/units.nix { inherit lib pkgs; } "agent-signing" agentKey;

  yubikeyKeys = pkgs.writeShellApplication {
    name = "yubikey-keys";
    runtimeInputs = [ pkgs.openssh ];
    runtimeEnv = {
      joel_key = joelKey;
      agent_key = agentKey;
    };
    text = builtins.readFile ./yubikey-keys.sh;
  };
in
{
  # The YubiKeys' spares are this machine's login keys, so the guide keeps
  # making them.
  imports = [
    ../signing/home.nix
    ../yubikey-spare/home.nix
  ];

  jitsusama.signing.signer = {
    name = "yubikey-sign";
    sshKeygen = lib.getExe' pkgs.openssh "ssh-keygen";
    inherit joelKey;
    inherit agentKey;
    makesKeys = "yubikey-keys";
  };

  home.packages = [ yubikeyKeys ];

  systemd.user = {
    inherit (served) sockets services;
  };

  home.activation.yubikeySigningKeys = lib.hm.dag.entryAfter [ "writeBoundary" ] (
    lib.concatMapStrings
      (
        file:
        let
          quoted = lib.escapeShellArg file;
        in
        ''
          if [ ! -f ${quoted} ]; then
            warnEcho "yubikey-signing: "${quoted}" is missing; yubikey-keys makes it."
          fi
        ''
      )
      [
        joelKey
        agentKey
      ]
  );
}
