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
}
