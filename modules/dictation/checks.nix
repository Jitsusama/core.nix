# The checks of dictation, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
{
  # A switch restarts dictation after it reloads the units, so the service
  # the shell's watch reaches is never the one from before the switch.
  dictation-switches-to-the-new-one =
    holds "dictation-switches-to-the-new-one"
      "A switch stops dictation before reloading, so the shell's watch starts the old one again."
      (((account graphical).systemd.user.services.dictation.Unit.X-SwitchMethod or null) == "restart");

  # The margin's "Stop and type it" wears the chord that dictates, so the
  # action Listening.qml looks for is the one the bind runs.
  dictation-stops-by-its-chord =
    let
      bound =
        lib.any (b: lib.hasInfix ''spawn "dictation" "hold";'' b.action)
          (account graphical).jitsusama.niri.binds;
      sought = lib.hasInfix ''"dictation hold"'' (builtins.readFile ./Listening.qml);
    in
    holds "dictation-stops-by-its-chord"
      "The margin's dictation piece looks for an action no chord runs, so its stop shows no chord."
      (bound && sought);

  # dictation's tests run as it builds: its spectrogram matches the one that
  # transcribed a clip word for word, its transform the plain sum, its
  # vocabulary reads back into text, and silence or a moment isn't heard.
  dictation-works-it-out = pkgs.callPackage ./package.nix { };

  # The whole of dictation's Whisper, on the CPU as a machine with no NPU
  # runs it: whisper.cpp's public clip, read and typed back word for word.
  dictation-hears-the-clip =
    let
      dictation = pkgs.callPackage ./package.nix { };
      model = pkgs.callPackage ./model.nix { };
      clip = "${pkgs.whisper-cpp.src}/samples/jfk.wav";
      want = lib.concatStringsSep " " [
        "And so my fellow Americans, ask not what your country can do for you,"
        "ask what you can do for your country."
      ];
    in
    pkgs.runCommand "dictation-hears-the-clip" { } ''
      heard=$(DICTATION_DEVICE=CPU ${lib.getExe dictation} transcribe ${model} ${clip})
      if [ "$heard" != ${lib.escapeShellArg want} ]; then
        echo "dictation heard: $heard"
        exit 1
      fi
      touch $out
    '';
}
// onLinux {
  # clippy finds nothing in dictation, its tests included.
  dictation-passes-clippy = passesClippy "dictation-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
