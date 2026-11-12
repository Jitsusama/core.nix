# The systemd user units that serve one SSH key kept in a file, on a machine
# with no TPM, at `<name>.sock` in the runtime folder: what ssh-tpm-agent's
# own socket and service are for a key in the TPM (../tpm-signing,
# ../link-keys), so whatever reaches a key through `<name>.socket`, such as
# mycelium's node, needn't know where the key is kept.
#
# ssh-agent can't take a socket systemd opened, so the socket starts a
# proxy (systemd-socket-proxyd) to an ssh-agent of the key's own, which
# binds a socket beside it and forks once it's bound, so the key is added
# before anything is served. It holds only that key and asks nothing.
#
# Both sit in agents.slice, as the TPM's agents do: what they sign is the
# agents' work, not Joel's programs'. agents.slice's oomd kills under
# pressure, so both ask to be spared, as a killed agent loses its key.
{ lib, pkgs }:
name: key:
let
  inner = "${name}-agent";
  sshAgent = lib.getExe' pkgs.openssh "ssh-agent";
  sshAdd = lib.getExe' pkgs.openssh "ssh-add";
  env = lib.getExe' pkgs.coreutils "env";
  rm = lib.getExe' pkgs.coreutils "rm";
in
{
  sockets.${name} = {
    Unit.Description = "The ${name} key's own agent, which asks nothing";
    Socket = {
      ListenStream = "%t/${name}.sock";
      SocketMode = "0600";
      Service = "${name}.service";
    };
    Install.WantedBy = [ "sockets.target" ];
  };

  services.${name} = {
    Unit = {
      Description = "The ${name} key's own agent, which asks nothing";
      Requires = [
        "${name}.socket"
        "${inner}.service"
      ];
      After = [
        "${name}.socket"
        "${inner}.service"
      ];
    };
    Service = {
      ExecStart = "${pkgs.systemd}/lib/systemd/systemd-socket-proxyd %t/${inner}.sock";
      Slice = "agents.slice";
      ManagedOOMPreference = "avoid";
    };
  };

  services.${inner} = {
    Unit = {
      Description = "The ssh-agent holding the ${name} key alone";
      ConditionPathExists = key;
      StopWhenUnneeded = true;
    };
    Service = {
      # ssh-agent forks once its socket is bound, which tells systemd it's
      # ready, so the key goes in before the proxy reaches it. A socket
      # left by an agent that died would stop it binding.
      Type = "forking";
      ExecStartPre = "${rm} -f %t/${inner}.sock";
      ExecStart = "${sshAgent} -a %t/${inner}.sock";
      ExecStartPost = "${env} SSH_AUTH_SOCK=%t/${inner}.sock ${sshAdd} -q ${key}";
      Slice = "agents.slice";
      ManagedOOMPreference = "avoid";
    };
  };
}
