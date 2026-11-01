# Noise-cancelled microphones: every microphone is offered a second time,
# as "‹microphone›, Noise Cancelled", with the sounds around Joel's voice
# taken out on this machine, so he picks it in a call like any other
# microphone and nothing changes when he doesn't.
#
# The program is dictation's fourth, noise-cancelled, sharing its OpenVINO
# calls (modules/dictation/src/bin/noise-cancelled/main.rs says how), with
# DeepFilterNet 3 exported a frame at a time (model.nix) on the processor.
# It's a PipeWire client of its own: a microphone is heard only while
# something records from its noise-cancelled one, and the model is held
# only while something does, and let go a little after.
#
# The service runs in session.slice beside PipeWire, not background.slice:
# a call's voice passes through it as it's spoken, so it can't wait behind
# a build the way a transcript can.
{ pkgs, ... }:
let
  dictation = pkgs.callPackage ../dictation/package.nix { };
  model = pkgs.callPackage ./model.nix { };
in
{
  systemd.user.services.noise-cancelled = {
    Unit = {
      Description = "Offers each microphone with the noise around its voice taken out";
      After = [ "pipewire.service" ];
      BindsTo = [ "pipewire.service" ];
      X-SwitchMethod = "restart";
    };
    Service = {
      ExecStart = "${dictation}/bin/noise-cancelled serve ${model}";
      Slice = "session.slice";
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "pipewire.service" ];
  };
}
