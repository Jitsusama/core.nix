# The checks of ssh-tpm-agent, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  pkgs,
  self,
  workstation,
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

  # The agent holding Joel's keys reads its gate from where its unit says,
  # and it's built with its tests, which judge ./gate-cases.txt by ./gate.conf, so
  # this check builds only once the agent refuses what each case says.
  ssh-tpm-agent-refuses-agents =
    let
      user = account workstation;
      agent = user.services.ssh-tpm-agent.package;
      gate = "SSH_TPM_AGENT_GATE=${user.xdg.configHome}/ssh-tpm-agent/gate";
      told = holds "ssh-tpm-agent-is-told-of-its-gate" "ssh-tpm-agent isn't told of core's gate." (
        user.xdg.configFile."ssh-tpm-agent/gate".source == ./gate.conf
        && builtins.elem gate user.systemd.user.services.ssh-tpm-agent.Service.Environment
        && user.systemd.user.services.ssh-tpm-agent.Service.LimitCORE or null == 0
        && builtins.elem ./refuses-agents.patch agent.patches
        && agent.doCheck
      );
    in
    pkgs.runCommandLocal "ssh-tpm-agent-refuses-agents" { inherit agent told; } ''
      touch $out
    '';
}
