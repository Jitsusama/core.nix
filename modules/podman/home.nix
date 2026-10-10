# Points Docker's clients at the account's own Podman socket, which the NixOS
# module starts in every account's runtime directory. The shell expands the
# directory as it starts, since it isn't known before login.
{
  home.sessionVariables.DOCKER_HOST = "unix://$XDG_RUNTIME_DIR/podman/podman.sock";
}
