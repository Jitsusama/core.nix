# Asks the kernel where a pi host's busy command goes: a host's unit sits in
# agents.slice beside the sessions Joel types into, and every command its
# sessions start runs in it, so one runaway there once slowed them all. It
# also asks what a call does to the work beside it: the sorter hears a call
# through the account's PipeWire and holds build.slice and jobs.slice to a
# quarter of the processors till it ends, and keeps them held when
# something clears a quota mid-call, since on 16 November jobs.slice was
# found uncapped through a call with no check to say so. A VM has no
# microphone, so a silent source stands in for one, and a PulseAudio
# recorder copied under a call program's name stands in for the call, as
# tests/vm/sound/calls.nix does. A second machine, of two cores, names its
# second as its efficient one, to ask where agents' work and builds run
# then (modules/slices/efficient-work.nix). The tests are
# hosts-vm-test.py, which pytest runs inside the driver.
{ self, pkgs }:
let
  # Processors enough that a quarter of them is less than all, and as many
  # spinners for the busy-work test, so its session has none to spare.
  processors = 4;
  # The programs the tests' units run, which they read from this file as
  # their values fixture.
  values = pkgs.writeText "hosts-values.json" (
    builtins.toJSON {
      sleep = "${pkgs.coreutils}/bin/sleep";
      bash = "${pkgs.bash}/bin/bash";
      setsid = "${pkgs.util-linux}/bin/setsid";
      wakes = "${./slices-foreground-wakes.sh}";
      pacat = "${pkgs.pulseaudio}/bin/pacat";
      pwDump = "${pkgs.pipewire}/bin/pw-dump";
      jq = "${pkgs.jq}/bin/jq";
      inherit processors;
    }
  );
in
import ../../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "hosts";

  nodes.machine = {
    imports = [
      self.nixosModules.slices
      self.nixosModules.pipewire
    ];
    virtualisation.cores = processors;
    users.users.alice = {
      isNormalUser = true;
      linger = true;
      # The machine's administrators may brake build.slice, as Joel is one
      # (work-slices.rules).
      extraGroups = [ "wheel" ];
    };
    # Builds' slice, which the Nix daemon's module declares on a machine.
    systemd.slices.build.sliceConfig.CPUWeight = 10;
    services.pipewire.extraConfig.pipewire."10-silent-microphone"."context.objects" = [
      {
        factory = "adapter";
        args = {
          "factory.name" = "support.null-audio-sink";
          "node.name" = "silent-microphone";
          # PipeWire's own recipe for a source with no device under it.
          "media.class" = "Audio/Source/Virtual";
          "audio.position" = [ "MONO" ];
          "object.linger" = true;
        };
      }
    ];
  };

  # Its own machine, since naming efficient cores on the first would leave
  # the busy-work test's session cores the spinners can't reach.
  nodes.pinned = {
    imports = [ self.nixosModules.slices ];
    virtualisation.cores = 2;
    jitsusama.slices.efficientCores = "1";
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
    systemd.slices.build.sliceConfig.CPUWeight = 10;
  };

  tests = ./hosts-vm-test.py;
  inherit values;
}
