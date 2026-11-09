# asahi-audio makes an Apple silicon Mac's speakers sound as they do on
# macOS: PipeWire filters tuned for each model's speakers, and the ALSA
# use-case files that name its outputs. It plays only beside speakersafetyd,
# which keeps the speakers safe, so it brings it. It follows
# nixos-apple-silicon's sound module (MIT, under the notice in
# LICENSE.nixos-apple-silicon).
{ pkgs, ... }:
let
  ucm = "${pkgs.alsa-ucm-conf-asahi}/share/alsa/ucm2";
in
{
  imports = [
    ../pipewire/nixos.nix
    ../speakersafetyd/nixos.nix
  ];

  # PulseAudio can't load asahi-audio's filters.
  services.pulseaudio.enable = false;

  services.pipewire = {
    configPackages = [ pkgs.asahi-audio ];
    wireplumber.configPackages = [ pkgs.asahi-audio ];
  };

  # Asahi's use-case files, for every program and for PipeWire and
  # WirePlumber, whether they run for a person or for the system.
  environment.variables.ALSA_CONFIG_UCM2 = ucm;
  systemd.user.services.pipewire.environment.ALSA_CONFIG_UCM2 = ucm;
  systemd.user.services.wireplumber.environment.ALSA_CONFIG_UCM2 = ucm;
  systemd.services.pipewire.environment.ALSA_CONFIG_UCM2 = ucm;
  systemd.services.wireplumber.environment.ALSA_CONFIG_UCM2 = ucm;
}
