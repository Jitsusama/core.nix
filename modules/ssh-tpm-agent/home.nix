# ssh-tpm-agent holds SSH keys sealed in the TPM, so no private key ever sits
# on the disk, and it serves every SSH login and SSH-signed commit. When a key
# wants its PIN or a confirmation, the agent runs SSH_ASKPASS, which the
# desktop's shell answers (modules/quickshell). The keys are made on the
# machine itself: jitsusama.tpmKeys names them, which the guide makes on a
# machine it installed (modules/guide/src/installed/keys.rs), and
# docs/installing.md says how to make them by hand. The login key's spare
# lives on each YubiKey, for when the TPM is wiped or the laptop is gone,
# which the guide keeps there too (modules/guide/src/installed/spares.rs).
{
  config,
  lib,
  pkgs,
  ...
}:
{
  options.jitsusama.tpmKeys = lib.mkOption {
    type = lib.types.listOf (
      lib.types.submodule {
        options = {
          file = lib.mkOption {
            type = lib.types.str;
            description = "Where the key is kept, without the .tpm and .pub ssh-tpm-keygen adds.";
          };
          label = lib.mkOption {
            type = lib.types.str;
            default = "";
            example = "signing";
            description = "What its comment says it's for, after the account and the machine.";
          };
          pin = lib.mkOption {
            type = lib.types.bool;
            description = "Whether it asks for a PIN before it's used.";
          };
          github = lib.mkOption {
            type = lib.types.nullOr (
              lib.types.enum [
                "authentication"
                "signing"
              ]
            );
            default = null;
            description = "What `guide github` adds it to GitHub as: a key that signs in, or one that signs commits.";
          };
        };
      }
    );
    default = [ ];
    description = ''
      The SSH keys this account keeps in the TPM, each named by the module
      that uses it. The guide makes each one missing on a machine it
      installed, as the account, with one PIN for every key that has one.
    '';
  };

  options.jitsusama.yubikeySpare = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = ''
      Whether the account keeps a spare of its login key on each of its
      YubiKeys, resident so `ssh-keygen -K` brings it back on any machine.
      The guide keeps one on each YubiKey a machine it installed names,
      downloading one that's there rather than making it again.
    '';
  };

  config = {
    services.ssh-tpm-agent.enable = true;

    # The login key, which the agent loads from ~/.ssh when it starts and
    # which asks for its PIN once a session.
    jitsusama.tpmKeys = [
      {
        file = "${config.home.homeDirectory}/.ssh/id_ecdsa";
        pin = true;
        github = "authentication";
      }
    ];

    # The agent starts serving before it holds the keys it read from ~/.ssh, so
    # the request that wakes it through its socket finds none, and the first
    # signature after login failed. The patch is the change upstream needs; take
    # it out once a release carries it.
    #
    # Patched, it builds here rather than coming from the cache, and four of its
    # tests check which error the kernel's keyring gives for a missing key. That
    # depends on the kernel: nixpkgs rewrites them to expect ENOENT, and some
    # kernels give ENOKEY, so they fail there whatever the agent does. They test
    # the kernel, not the agent, so they're skipped. The pattern has no
    # parentheses or question marks: the build's shell reads those as a glob and
    # drops the flag without a word.
    services.ssh-tpm-agent.package = pkgs.ssh-tpm-agent.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [ ./keys-before-serving.patch ];
      checkFlags = (old.checkFlags or [ ]) ++ [
        "-skip=^TestNoKey$|^TestRemoveKey$|^TestNoKeyThreaded$|^TestRemoveKeyThreaded$"
      ];
    });
  };
}
