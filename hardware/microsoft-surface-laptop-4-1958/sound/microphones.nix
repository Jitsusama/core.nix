# The Surface Laptop 4's built-in microphones, at a level a call can use.
# WirePlumber starts an input nobody has set at full volume, and PipeWire's
# path for an internal mic makes full volume every gain the codec has, so
# everything they heard clipped. The path in paths/ holds their gain at
# the mics' own level. A path is chosen through the card's profile set, so
# the laptop's analog card gets PipeWire's analog stereo profile with only
# that path swapped, under PipeWire's own name, so the input the desktop
# shows keeps its name; the headset jack and the outputs keep PipeWire's.
{ pkgs, ... }:
let
  # PipeWire's default.conf mapping for analog stereo, as of PipeWire 1.6.
  profileSet = pkgs.writeText "microsoft-surface-laptop-4.conf" ''
    [General]
    auto-profiles = yes

    [Mapping analog-stereo]
    device-strings = front:%f
    channel-map = left,right
    paths-output = analog-output analog-output-lineout analog-output-speaker analog-output-headphones analog-output-headphones-2
    paths-input = analog-input-front-mic analog-input-rear-mic ${./paths}/analog-input-internal-mic analog-input-dock-mic analog-input analog-input-mic analog-input-linein analog-input-aux analog-input-video analog-input-tvtuner analog-input-fm analog-input-mic-line analog-input-headphone-mic analog-input-headset-mic
    priority = 15
  '';
in
{
  imports = [ ../../../modules/pipewire/nixos.nix ];

  services.pipewire.wireplumber.extraConfig."50-surface-microphones"."monitor.alsa.rules" = [
    {
      matches = [ { "device.name" = "alsa_card.pci-0000_03_00.6"; } ];
      actions.update-props."device.profile-set" = "${profileSet}";
    }
  ];
}
