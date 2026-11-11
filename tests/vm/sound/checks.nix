# The sound VM tests' checks, which checks.nix gathers with every other
# folder's; README.md beside this says what each test proves.
{
  self,
  onLinux,
  vmHostPkgs,
  ...
}:
let
  pkgs = vmHostPkgs;
in
onLinux {
  vm-speakers-are-tuned = import ./speakers.nix { inherit pkgs; };
  # Only a running PipeWire, another account's, shows root reads it.
  vm-a-call-is-on-reads-every-account = import ./calls.nix { inherit pkgs; };
  # Only a running PipeWire with a call in it shows what's recorded of each side.
  vm-a-call-is-written-down = import ./call-transcripts.nix { inherit self pkgs; };
  # Only a running PipeWire shows a microphone offered, heard only while recorded.
  vm-a-microphone-is-noise-cancelled = import ./noise-cancelled.nix {
    inherit self pkgs;
  };
  vm-pebble-speakers-are-tuned = import ./pebble-x-plus.nix { inherit pkgs; };
  vm-mv7-voice-is-levelled = import ./shure-mv7-voice.nix { inherit pkgs; };
  # Which input WirePlumber picks comes from the rank it gives each
  # microphone as udev reports its card, so only a running machine shows it.
  vm-webcam-microphones-yield = import ./microphones.nix { inherit pkgs; };
  # Whether dictation's chord is heard let go, and nothing else of the
  # keys, shows only with a keyboard there and an account outside its group.
  vm-dictation-hears-the-chord-let-go = import ./dictation-chord.nix {
    inherit self pkgs;
  };
  # When dictation says it's listening shows only with a sound server there,
  # with a microphone and without.
  vm-dictation-listens-once-it-hears = import ./dictation-listening.nix {
    inherit self pkgs;
  };
}
