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

  # The scorer beside Joel's ear, on a noisy clip and the same cleaned:
  # tests/a-cleaned-clip-scores-better.sh says what it proves.
  noise-cancelled-scores-a-cleaned-clip-better =
    let
      dictation = pkgs.callPackage ../dictation/package.nix { };
      model = pkgs.callPackage ./model.nix { };
      models = pkgs.callPackage ./models.nix { };
    in
    pkgs.runCommand "noise-cancelled-scores-a-cleaned-clip-better"
      {
        nativeBuildInputs = [
          pkgs.bc
          pkgs.jq
          pkgs.sox
        ];
      }
      ''
        $BASH ${./tests/a-cleaned-clip-scores-better.sh} ${dictation}/bin/noise-cancelled ${models} \
          ${model.source}/assets/clean_freesound_33711.wav \
          ${model.source}/assets/noise_freesound_573577.wav
        touch $out
      '';
}
