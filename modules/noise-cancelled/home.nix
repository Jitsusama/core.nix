# Noise-cancelled microphones: every microphone is offered a second time,
# as "‹microphone›, Noise Cancelled", with the sounds around Joel's voice
# taken out on this machine, so he picks it in a call like any other
# microphone and nothing changes when he doesn't.
#
# The program is dictation's fourth, noise-cancelled, sharing its OpenVINO
# calls (modules/dictation/src/bin/noise-cancelled/main.rs says how), with
# DeepFilterNet 3 exported a frame at a time (model.nix) on the processor,
# or RNNoise, as jitsusama.noise-cancelled.cleaner says.
# It's a PipeWire client of its own: a microphone is heard only while
# something records from its noise-cancelled one, and the model is held
# only while something does, and let go a little after.
#
# The service runs in session.slice beside PipeWire, not background.slice:
# a call's voice passes through it as it's spoken, so it can't wait behind
# a build the way a transcript can.
#
# Choose by Ear, in the launcher's Sound, is the same program's trial: it
# records six seconds from the default microphone, cleans them every way,
# and plays them back blind for Joel to pick one (trial.rs says how). It
# starts from its socket, holds what he said only in memory, and may reach
# no network. It records nothing while a call is on, or while anything in
# jitsusama.noise-cancelled.refuseWhile says not to. A cleaned pick is kept
# in ~/.local/state/noise-cancelled/cleaner, which the service reads each
# time it loads the model, so his next call cleans as he picked, in place
# of jitsusama.noise-cancelled.cleaner, with nothing restarted.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.noise-cancelled;
  dictation = pkgs.callPackage ../dictation/package.nix { };
  model = pkgs.callPackage ./model.nix { };
  # Where a pick by ear is kept, in the account's state folder (%S).
  chosen = "%S/noise-cancelled/cleaner";
  refusals = pkgs.writeText "noise-cancelled-refusals.json" (
    builtins.toJSON (map (refusal: { inherit (refusal) command why; }) cfg.refuseWhile)
  );
in
{
  options.jitsusama.noise-cancelled.cleaner = lib.mkOption {
    type = lib.types.enum [
      "deepfilternet"
      "deepfilternet-gentle"
      "rnnoise"
    ];
    default = "deepfilternet";
    description = ''
      How the noise-cancelled microphones clean a voice until Joel picks one
      by ear, which wins from then on: deepfilternet takes the room out,
      deepfilternet-gentle turns it down to 12 dB under the voice, and
      rnnoise is the lightest, at a fraction of the processor.
    '';
  };

  options.jitsusama.noise-cancelled.refuseWhile = lib.mkOption {
    type = lib.types.listOf (
      lib.types.submodule {
        options = {
          command = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            description = "A command that exits 0 while nothing may be recorded.";
          };
          why = lib.mkOption {
            type = lib.types.str;
            description = "Why, as the launcher says it, such as \"a meeting is near\".";
          };
        };
      }
    );
    default = [ ];
    description = ''
      What refuses Choose by Ear a recording while it holds, beside a call
      being on, which is always refused.
    '';
  };

  config.jitsusama.noise-cancelled.refuseWhile = lib.mkBefore [
    {
      command = [ (lib.getExe (pkgs.callPackage ../calls/package.nix { })) ];
      why = "a call is on";
    }
  ];

  config.systemd.user.sockets.noise-cancelled-trial = {
    Unit.Description = "Where the launcher's Choose by Ear asks noise-cancelled";
    Socket = {
      ListenStream = "%t/daemons/noise-cancelled.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };

  config.systemd.user.services.noise-cancelled-trial = {
    Unit = {
      Description = "Records, cleans and plays back the launcher's Choose by Ear";
      After = [ "pipewire.service" ];
    };
    Service = {
      ExecStart = lib.concatStringsSep " " [
        "${dictation}/bin/noise-cancelled trial %t/daemons/noise-cancelled.sock"
        "${model} ${cfg.cleaner} ${refusals} ${chosen}"
      ];
      StateDirectory = "noise-cancelled";
      Environment = [ "PATH=${lib.makeBinPath [ pkgs.pipewire ]}" ];
      Slice = "session.slice";
      # What Joel said reaches nothing past PipeWire's own socket.
      RestrictAddressFamilies = "AF_UNIX";
      Restart = "on-failure";
    };
  };

  config.systemd.user.services.noise-cancelled = {
    Unit = {
      Description = "Offers each microphone with the noise around its voice taken out";
      After = [ "pipewire.service" ];
      BindsTo = [ "pipewire.service" ];
      X-SwitchMethod = "restart";
    };
    Service = {
      ExecStart = "${dictation}/bin/noise-cancelled serve ${model} ${cfg.cleaner} ${chosen}";
      Slice = "session.slice";
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "pipewire.service" ];
  };
}
