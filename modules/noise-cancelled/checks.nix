# The checks of noise-cancelled, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves. The
# program's own checks are dictation's, whose crate it's built in.
{
  account,
  graphical,
  holds,
  onLinux,
  pkgs,
  ...
}:
{
  # A switch restarts it after it reloads the units, so the microphones
  # offered are never the old program's.
  noise-cancelled-switches-to-the-new-one =
    holds "noise-cancelled-switches-to-the-new-one"
      "A switch stops noise-cancelled before reloading, so the old one would keep offering microphones."
      (
        ((account graphical).systemd.user.services.noise-cancelled.Unit.X-SwitchMethod or null) == "restart"
      );

  # After a call the service scores what every cleaner would have made of
  # its first speech, so it holds Joel's voice as Choose by Ear does, and
  # may reach no more than it.
  noise-cancelled-reaches-no-network =
    holds "noise-cancelled-reaches-no-network"
      "noise-cancelled could reach a network with a call's speech in its memory."
      (
        ((account graphical).systemd.user.services.noise-cancelled.Service.RestrictAddressFamilies
          or null
        ) == "AF_UNIX"
      );

  # Choose by Ear holds a few seconds of Joel's voice; the service that
  # holds them may reach nothing but local sockets.
  noise-cancelled-trial-reaches-no-network =
    holds "noise-cancelled-trial-reaches-no-network"
      "Choose by Ear's service could reach a network with Joel's voice in its memory."
      (
        ((account graphical).systemd.user.services.noise-cancelled-trial.Service.RestrictAddressFamilies
          or null
        ) == "AF_UNIX"
      );

  # After each call the service scores what every cleaner would have made
  # of it, so it needs the scorer's model beside the cleaner's, as the
  # trial has them. Given the cleaner's folder alone, every call's score
  # failed to load and no call was judged.
  noise-cancelled-scores-with-the-trials-models =
    let
      services = (account graphical).systemd.user.services;
      words = service: pkgs.lib.splitString " " (toString service.Service.ExecStart);
      served = builtins.elemAt (words services.noise-cancelled) 2;
    in
    # The folder is named for models.nix, so a word moved out of place
    # in both commands can't pass by matching some other word.
    holds "noise-cancelled-scores-with-the-trials-models"
      "noise-cancelled serve is given another models folder than the trial, so it can't score a call."
      (
        pkgs.lib.hasSuffix "-noise-cancelled-models" served
        && served == builtins.elemAt (words services.noise-cancelled-trial) 3
      );

  # Whatever else a machine refuses it for, Choose by Ear never records
  # during a call, which it would cut into.
  noise-cancelled-trial-waits-out-a-call =
    holds "noise-cancelled-trial-waits-out-a-call" "Choose by Ear could record while a call is on."
      (
        builtins.any (refusal: refusal.why == "a call is on")
          (account graphical).jitsusama.noise-cancelled.refuseWhile
      );
}
// onLinux {
  # The denoiser on a noisy clip, on the CPU, in the build's sandbox:
  # tests/a-noisy-clip-comes-out-clean.sh says what it proves.
  noise-cancelled-cleans-a-noisy-clip =
    let
      dictation = pkgs.callPackage ../dictation/package.nix { };
      model = pkgs.callPackage ./model.nix { };
    in
    pkgs.runCommand "noise-cancelled-cleans-a-noisy-clip"
      {
        # The sandbox has no NPU, so the frames go first to the processor
        # as they would to it, started without waiting, with the processor
        # behind for those late; the other checks clean on the processor
        # alone, as where the NPU won't compile.
        NOISE_CANCELLED_DEVICE = "CPU";
        nativeBuildInputs = [
          pkgs.bc
          pkgs.sox
        ];
      }
      ''
        $BASH ${./tests/a-noisy-clip-comes-out-clean.sh} ${dictation}/bin/noise-cancelled ${model} \
          ${model.source}/assets/clean_freesound_33711.wav \
          ${model.source}/assets/noise_freesound_573577.wav
        touch $out
      '';

  # Every cleaner, as the scorer hears it, in two noises:
  # tests/every-cleaner-keeps-the-voice.sh says what it proves.
  noise-cancelled-keeps-the-voice-it-cleans =
    let
      dictation = pkgs.callPackage ../dictation/package.nix { };
      model = pkgs.callPackage ./model.nix { };
      models = pkgs.callPackage ./models.nix { };
    in
    pkgs.runCommand "noise-cancelled-keeps-the-voice-it-cleans"
      {
        nativeBuildInputs = [
          pkgs.bc
          pkgs.jq
          pkgs.sox
        ];
      }
      ''
        $BASH ${./tests/every-cleaner-keeps-the-voice.sh} ${./tests/clips.sh} \
          ${dictation}/bin/noise-cancelled ${models} \
          ${model.source}/assets/clean_freesound_33711.wav \
          ${model.source}/assets/noise_freesound_573577.wav
        touch $out
      '';

  # The scorer beside Joel's ear, on a voice, the same voice in noise, and
  # that cleaned: tests/a-clean-voice-scores-better.sh says what it proves.
  noise-cancelled-scores-a-clean-voice-better =
    let
      dictation = pkgs.callPackage ../dictation/package.nix { };
      model = pkgs.callPackage ./model.nix { };
      models = pkgs.callPackage ./models.nix { };
    in
    pkgs.runCommand "noise-cancelled-scores-a-clean-voice-better"
      {
        nativeBuildInputs = [
          pkgs.bc
          pkgs.jq
          pkgs.sox
        ];
      }
      ''
        $BASH ${./tests/a-clean-voice-scores-better.sh} ${./tests/clips.sh} \
          ${dictation}/bin/noise-cancelled ${models} \
          ${model.source}/assets/clean_freesound_33711.wav \
          ${model.source}/assets/noise_freesound_573577.wav
        touch $out
      '';
}
