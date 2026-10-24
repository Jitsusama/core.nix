# The checks of ssh-tpm-agent, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  self,
  ...
}:
{
  # A workstation with a screen has one SSH agent, the TPM's, though niri
  # brings gnome-keyring and its agent.
  one-ssh-agent =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
      ];
    in
    holds "one-ssh-agent" "gcr's SSH agent runs beside ssh-tpm-agent." (
      !machine.config.services.gnome.gcr-ssh-agent.enable
      && (account machine).services.ssh-tpm-agent.enable
    );
}
