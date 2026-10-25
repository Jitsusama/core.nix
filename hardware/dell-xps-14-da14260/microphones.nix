# The XPS 14's built-in microphones, kept ready once opened. WirePlumber
# closes a source five seconds after its last reader goes, and opening the
# laptop's again starts its DSP's capture pipeline over SoundWire, which
# took 690 ms against 165 ms for one still running. Dictation records from
# a key press, so those 690 ms were the start of what was said. Keeping the
# audio DSP itself powered didn't shorten it; keeping the source running
# does. Until a program first opens it after sign-in it still starts slowly.
{
  imports = [ ../../modules/pipewire/nixos.nix ];

  services.pipewire.wireplumber.extraConfig."50-microphones-stay-ready"."monitor.alsa.rules" = [
    {
      matches = [ { "node.name" = "~alsa_input.*sof_sdw.*Mic.*"; } ];
      actions.update-props."session.suspend-timeout-seconds" = 0;
    }
  ];
}
