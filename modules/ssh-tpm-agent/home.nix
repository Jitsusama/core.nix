# ssh-tpm-agent holds SSH keys sealed in the TPM, so no private key ever sits
# on the disk, and it serves every SSH login and SSH-signed commit. When a key
# wants its PIN or a confirmation, the agent runs SSH_ASKPASS, which the
# desktop's shell answers (modules/quickshell). The keys are made on the
# machine itself; see docs/installing.md.
{ pkgs, ... }:
{
  services.ssh-tpm-agent.enable = true;

  # The agent starts serving before it holds the keys it read from ~/.ssh, so
  # the request that wakes it through its socket finds none, and the first
  # signature after login failed. The patch is the change upstream needs; take
  # it out once a release carries it.
  #
  # Patched, it builds here rather than coming from the cache, and four of its
  # tests check which error the kernel's keyring gives for a missing key. That
  # depends on the kernel: nixpkgs rewrites them to expect ENOENT, and CI's
  # runners give ENOKEY, so they fail there whatever the agent does. They test
  # the kernel, not the agent, so they're skipped. The pattern has no
  # parentheses or question marks: the build's shell reads those as a glob and
  # drops the flag without a word.
  services.ssh-tpm-agent.package = pkgs.ssh-tpm-agent.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [ ./keys-before-serving.patch ];
    checkFlags = (old.checkFlags or [ ]) ++ [
      "-skip=^TestNoKey$|^TestRemoveKey$|^TestNoKeyThreaded$|^TestRemoveKeyThreaded$"
    ];
  });
}
