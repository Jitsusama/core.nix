# pi's part of a NixOS machine's own config, all of it its agent proxy's:
# ssh offers the key the proxy makes for OS Login hosts (src/os_login.rs) to
# each instance gcloud reaches through its IAP tunnel, which it names
# compute.<id>. gcloud's ssh offers only the person's own key, sealed in
# the TPM, whose agent refuses agents, so on 24 November an agent's
# `gcloud compute ssh` failed. With this, an agent's ssh, whose agent is
# the proxy, offers the proxy's key too, and only the proxy holds it.
#
# It's the machine's ssh_config, since the account's own ~/.ssh/config is
# the person's and gcloud's to write. A machine where no proxy runs, or
# none has written its key, has no such file, and ssh passes over an
# IdentityFile the config names that isn't there without a word. The
# person's own ssh offers their key first, as gcloud names it; only when
# that fails does ssh say it can't load this one, which its own agent
# doesn't hold. A host named compute. otherwise gains only the offer: the
# proxy signs with the key for nothing but an instance gcloud knows.
#
# A login gcloud makes to an instance's address rather than through IAP
# names the address, so it isn't offered there; agents reach instances
# through IAP. The block ends with `Host *`, so whatever another module
# adds after it applies to every host, as it would have without it.
{
  programs.ssh.extraConfig = ''
    Host compute.*
      IdentityFile /run/user/%i/pi-agent/os-login.pub
    Host *
  '';
}
