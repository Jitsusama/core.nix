# Containers through Podman, run by each account itself: no daemon running as
# root and no group that grants it. The docker command is Podman's, and each
# account's own Podman socket answers the Docker API, so tools that insist on
# Docker, or on DOCKER_HOST, work unchanged; home.nix points DOCKER_HOST at it.
{ pkgs, ... }:
{
  virtualisation.podman = {
    enable = true;
    dockerCompat = true;
    # Containers on the default network find each other by name.
    defaultNetwork.settings.dns_enabled = true;
  };

  # `podman compose` and `docker compose` hand compose files to this.
  environment.systemPackages = [ pkgs.docker-compose ];
}
