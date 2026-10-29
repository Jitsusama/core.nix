# Joel's commits and tags are signed, on every machine, with one of two
# keys held in its hardware, where they can't be copied out:
#
#   joel-signing   asks first (a PIN or Touch ID), then on every signature
#   agent-signing  never asks, for pi and the other harnesses
#
# The hardware differs, so a Linux machine's TPM (../tpm-signing) or a Mac's
# Secure Enclave (../secure-enclave-signing) imports this and says, in
# jitsusama.signing.signer, how its ssh-keygen reaches the keys. What's the
# same lives here: git's settings, the register of allowed signers, and the
# signing program (sign.sh), which picks a key each time it signs. Imported
# alone, as a machine repository's modules that list the signers do, it
# signs nothing until hardware's module says how.
#
# When the program's standard input is a terminal and it can open its
# controlling terminal, a person is at the keyboard and Joel's key signs;
# otherwise a harness is driving git over pipes and the agents' key signs.
# git hands the signer its own standard input, so this is the input of `git
# commit` itself. A harness started from a terminal can still open /dev/tty,
# which is why both are checked. SIGN_AS=joel or SIGN_AS=agent decides it
# outright.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.signing;
  inherit (cfg) signer;

  # Signs with the key for whoever drives git (sign.sh), given both.
  program = pkgs.writeShellApplication {
    inherit (signer) name;
    text = lib.concatLines (
      [
        (lib.concatStringsSep " " [
          "name=${lib.escapeShellArg signer.name}"
          "real=${lib.escapeShellArg signer.sshKeygen}"
          "joel_key=${lib.escapeShellArg signer.joelKey}"
          "agent_key=${lib.escapeShellArg signer.agentKey}"
          "makes_keys=${lib.escapeShellArg signer.makesKeys}"
        ])
      ]
      ++ lib.mapAttrsToList (variable: value: "export ${variable}=\"${value}\"") signer.environment
      ++ [
        (lib.concatStringsSep "\n" (
          [ "agent_environment() {" ]
          ++ lib.mapAttrsToList (variable: value: "  export ${variable}=\"${value}\"") signer.agentEnvironment
          ++ [
            "  :"
            "}"
          ]
        ))
        (builtins.readFile ./sign.sh)
      ]
    );
  };
in
{
  imports = [ ../git/home.nix ];

  options.jitsusama.signing = {
    allowedSigners = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ ''joel@example.com namespaces="git" ecdsa-sha2-nistp256 AAAA...'' ];
      description = ''
        Lines of git's allowed_signers file: every key that has signed Joel's
        work, on every machine, so git can say whose a signature is. It's a
        register that only grows; a retired key gets valid-before="YYYYMMDD"
        rather than being removed, so what it signed still verifies.
      '';
    };

    signer = lib.mkOption {
      internal = true;
      description = ''
        How this machine's hardware signs, which the module for that hardware
        sets, never a machine; nothing is signed while it's null.
      '';
      default = null;
      type = lib.types.nullOr (
        lib.types.submodule {
          options = {
            name = lib.mkOption {
              type = lib.types.str;
              description = "The signing program's name, which a prompt shows as what asked.";
            };
            sshKeygen = lib.mkOption {
              type = lib.types.str;
              description = "The ssh-keygen that signs and verifies.";
            };
            joelKey = lib.mkOption {
              type = lib.types.str;
              description = "What ssh-keygen's -f takes to sign with Joel's key, which git names too.";
            };
            agentKey = lib.mkOption {
              type = lib.types.str;
              description = "What ssh-keygen's -f takes to sign with the agents' key.";
            };
            makesKeys = lib.mkOption {
              type = lib.types.str;
              description = "What makes a key that's missing, as the program says when one is.";
            };
            environment = lib.mkOption {
              type = lib.types.attrsOf lib.types.str;
              default = { };
              description = ''
                Variables the ssh-keygen needs to reach the keys, each exported
                in double quotes when the program runs, so a value may expand
                another variable.
              '';
            };
            agentEnvironment = lib.mkOption {
              type = lib.types.attrsOf lib.types.str;
              default = { };
              description = ''
                Variables exported after the environment when the agents' key
                signs, as the environment's are, so it can reach a key held
                apart from Joel's, which no prompt of his holds up.
              '';
            };
          };
        }
      );
    };
  };

  config = lib.mkIf (signer != null) {
    home.packages = [ program ];

    # Last, so an include another tool writes, such as dev's YubiKey
    # settings, can't take signing back.
    programs.git.includes = lib.mkAfter [
      {
        # Recursive, since a plain // would put the signers' gpg section in
        # place of the one that says how to sign.
        contents =
          lib.recursiveUpdate
            {
              gpg.format = "ssh";
              gpg.ssh.program = lib.getExe program;
              user.signingKey = signer.joelKey;
              commit.gpgSign = true;
              tag.gpgSign = true;
            }
            (
              lib.optionalAttrs (cfg.allowedSigners != [ ]) {
                gpg.ssh.allowedSignersFile = "${config.xdg.configHome}/git/allowed_signers";
              }
            );
      }
    ];

    xdg.configFile."git/allowed_signers" = lib.mkIf (cfg.allowedSigners != [ ]) {
      text = lib.concatLines cfg.allowedSigners;
    };
  };
}
