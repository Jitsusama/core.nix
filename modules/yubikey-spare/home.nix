# Whether an account keeps a spare login key on each of its YubiKeys, which
# the guide makes (modules/guide/src/installed/spares.rs). It's its own
# module so a machine with no TPM, where the spares are the only login
# keys, keeps them without ssh-tpm-agent, which imports it too.
{ lib, ... }:
{
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
}
