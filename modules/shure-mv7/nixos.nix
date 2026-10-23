# Shure's MV7 microphone shares one mute, one gain and one headphone volume
# with the desktop. Its firmware names its controls Microphone and USB
# Streaming, which none of PipeWire's ready-made paths know, so PipeWire
# would mute and turn it down in software, apart from the mic: its touch pad
# muted a microphone the desktop still showed live, and the desktop's mute
# left the pad's light green. A profile set of its own names them, so
# muting from either side mutes both, the desktop's input volume is the
# mic's own gain, and its touch strip is the headphone volume the desktop
# shows. In the auto level mode it's left in, the mic ignores that gain, so
# the slider changes nothing a listener hears. Its voice is brought up to the level programs expect by a filter
# that voice.conf describes, run as a PipeWire client of its own in every
# account, following the sound server as the speakers' tunings do.
{ config, pkgs, ... }:
let
  # The input carries the higher priority so the profile with both wins
  # over the one with only the headphones.
  profileSet = pkgs.writeText "shure-mv7.conf" ''
    [General]
    auto-profiles = yes

    [Mapping analog-mono]
    device-strings = hw:%f
    channel-map = mono
    paths-input = ${./paths}/shure-mv7-microphone
    direction = input
    priority = 2

    [Mapping analog-stereo]
    device-strings = hw:%f
    channel-map = left,right
    paths-output = ${./paths}/shure-mv7-headphones
    direction = output
    priority = 1
  '';
in
{
  imports = [ ../pipewire/nixos.nix ];

  services.pipewire.wireplumber.extraConfig."50-shure-mv7"."monitor.alsa.rules" = [
    {
      matches = [
        {
          "device.vendor.id" = "0x14ed";
          "device.product.id" = "0x1012";
        }
      ];
      actions.update-props."device.profile-set" = "${profileSet}";
    }
  ];

  systemd.user.services.shure-mv7-voice = {
    description = "The Shure MV7's voice level";
    after = [
      "pipewire.service"
      "wireplumber.service"
    ];
    requires = [ "pipewire.service" ];
    wants = [ "wireplumber.service" ];
    partOf = [ "pipewire.service" ];
    wantedBy = [ "pipewire.service" ];
    # The limiter is an LV2 plugin, which PipeWire looks for here.
    environment.LV2_PATH = "${pkgs.lsp-plugins}/lib/lv2";
    serviceConfig = {
      ExecStart = "${config.services.pipewire.package}/bin/pipewire -c ${./voice.conf}";
      Restart = "on-failure";
      RestartSec = 2;
    };
  };
}
