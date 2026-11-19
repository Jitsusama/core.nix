# The checks of ssh-tpm-agent, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  graphical,
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
  # keeps no core, and leaves a zsh the agent it was given while the login's
  # services are pointed at it whatever they held; and it's built with its
  # tests, which judge ./gate-cases.txt by ./gate.conf, so this check builds
  # only once the agent refuses what each case says.
  ssh-tpm-agent-refuses-agents =
    let
      user = account workstation;
      agent = user.services.ssh-tpm-agent.package;
      gate = "SSH_TPM_AGENT_GATE=${user.xdg.configHome}/ssh-tpm-agent/gate";
      service = user.systemd.user.services.ssh-tpm-agent.Service;
      told = holds "ssh-tpm-agent-is-told-of-its-gate" "ssh-tpm-agent isn't told of core's gate." (
        user.xdg.configFile."ssh-tpm-agent/gate".source == ./gate.conf
        && user.xdg.configFile."ssh-tpm-agent/gate".force
        && builtins.elem gate service.Environment
        && builtins.elem ./refuses-agents.patch agent.patches
        && agent.doCheck
      );
      coreless = holds "ssh-tpm-agent-keeps-no-core" "ssh-tpm-agent may keep a core of its PIN." (
        service.LimitCORE or null == 0
      );
      # gate-cases.txt copies the slices' names by hand, so a slice renamed
      # would leave its cases passing and its work served: each slice the
      # gate names must still be one a machine with a screen makes, as the
      # graphical role brings the slices.
      awaited = graphical.config.systemd.user.services.awaited.serviceConfig;
      named =
        holds "ssh-tpm-agent-gate-names-real-slices"
          "ssh-tpm-agent's gate names a slice, or spares a service, the graphical role lacks."
          (
            # gate.conf spells out every name in the awaited slice but
            # awaited, so that must be the service the slices put there.
            (awaited.Slice or null) == "jobs-awaited.slice"
            && builtins.all (slice: graphical.config.systemd.user.slices ? ${slice}) [
              "agents"
              "background"
              "jobs"
              "jobs-awaited"
              "jobs-timed"
              "jobs-work"
              "shared"
            ]
          );
      inherit (user.sshAuthSock.initialization) bash zsh;
    in
    pkgs.runCommandLocal "ssh-tpm-agent-refuses-agents"
      {
        inherit
          agent
          told
          coreless
          named
          ;
        nativeBuildInputs = [ pkgs.zsh ];
        inherit bash zsh;
      }
      ''
        ${pkgs.bash}/bin/bash ${./agent-socket-check.sh} "$zsh" "$bash"
        touch $out
      '';
}
