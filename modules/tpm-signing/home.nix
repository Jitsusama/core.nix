# Joel's signing keys sealed in a Linux machine's TPM, which ssh-tpm-agent
# holds; ../signing says how a key is picked and what git is told. Joel's key
# asks for its PIN once a session, then for a confirmation on every
# signature.
#
# ssh-tpm-agent loads every key under ~/.ssh when it starts, without asking
# to confirm, so Joel's key lives outside it and the signing-key unit adds it
# with confirmation whenever the agent starts. jitsusama.tpmKeys names both,
# for the guide to make on a machine it installed, and docs/installing.md
# says how to make them by hand.
#
# The agents sign through an ssh-tpm-agent of their own, holding only their
# key, which asks nothing. ssh-tpm-agent signs nothing else while one of
# Joel's keys waits for his PIN or his yes, so an ssh that offered his login
# key while he was away once held every agent's commit up for twenty-three
# minutes, until the prompt was cancelled.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  joelKey = "${config.xdg.dataHome}/signing/joel-signing";
  agentKey = "${config.home.homeDirectory}/.ssh/agent-signing";
  sshTpmAdd = lib.getExe' config.services.ssh-tpm-agent.package "ssh-tpm-add";
  sshTpmAgent = lib.getExe config.services.ssh-tpm-agent.package;
  # The agents' agent loads every key in its folder, which holds only theirs,
  # linked from where the guide makes it, so it's made in one place.
  agentKeys = "signing/agent-keys";
  agentSocket = "agent-signing.sock";
in
{
  imports = [
    ../signing/home.nix
    ../ssh-tpm-agent/home.nix
  ];

  # ssh-keygen signs through the TPM's agent by each key's public half,
  # whatever agent SSH itself is using.
  jitsusama.signing.signer = {
    name = "tpm-sign";
    sshKeygen = lib.getExe' pkgs.openssh "ssh-keygen";
    joelKey = "${joelKey}.pub";
    agentKey = "${agentKey}.pub";
    makesKeys = "docs/installing.md's step 9";
    environment.SSH_AUTH_SOCK = "\${SSH_TPM_AUTH_SOCK:-$XDG_RUNTIME_DIR/ssh-tpm-agent.sock}";
    agentEnvironment.SSH_AUTH_SOCK = "$XDG_RUNTIME_DIR/${agentSocket}";
  };

  xdg.dataFile = lib.genAttrs [ "tpm" "pub" ] (kind: {
    target = "${agentKeys}/agent-signing.${kind}";
    source = config.lib.file.mkOutOfStoreSymlink "${agentKey}.${kind}";
  });

  # Started by the first signature that wants it, it loads the key before it
  # serves, as the agent's patch makes ssh-tpm-agent do (../ssh-tpm-agent).
  systemd.user.sockets.agent-signing = {
    Unit.Description = "The agents' signing key's own ssh-tpm-agent, which asks nothing";
    Socket = {
      ListenStream = "%t/${agentSocket}";
      SocketMode = "0600";
      Service = "agent-signing.service";
    };
    Install.WantedBy = [ "sockets.target" ];
  };
  systemd.user.services.agent-signing = {
    Unit = {
      Description = "The agents' signing key's own ssh-tpm-agent, which asks nothing";
      Requires = [ "agent-signing.socket" ];
      After = [ "agent-signing.socket" ];
    };
    Service = {
      Environment = "SSH_TPM_AUTH_SOCK=%t/${agentSocket}";
      ExecStart = "${sshTpmAgent} --key-dir ${config.xdg.dataHome}/${agentKeys}";
      # As ssh-tpm-agent's own unit has it.
      SuccessExitStatus = 2;
    };
  };

  jitsusama.tpmKeys = [
    {
      file = joelKey;
      label = "signing";
      pin = true;
      github = "signing";
    }
    {
      file = agentKey;
      label = "agents";
      pin = false;
      github = "signing";
    }
  ];

  systemd.user.services.signing-key = {
    Unit = {
      Description = "Joel's signing key in ssh-tpm-agent, confirmed on every use";
      After = [ "ssh-tpm-agent.service" ];
      PartOf = [ "ssh-tpm-agent.service" ];
      ConditionPathExists = "${joelKey}.tpm";
    };
    Service = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = "${sshTpmAdd} -c ${joelKey}.tpm";
    };
    Install.WantedBy = [ "ssh-tpm-agent.service" ];
  };
}
