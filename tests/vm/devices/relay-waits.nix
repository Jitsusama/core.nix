# How long the relay every shared camera has waits for a frame before it
# makes its camera's pipeline again, and what it does when the pipeline made
# again gives none either (../../../modules/v4l2loopback/relay.nix says why
# it's patched). What has to hold shows only with a loopback there and a
# program reading it: a client that has had no frame at all waits 90 s
# before the pipeline is made again, and as long again before the relay
# ends, saying so each time, for its service to start it afresh; a client
# whose frames stopped waits 10 s, and a camera that frames again after each
# is made again each time and never ended. No VM has a camera, so two inputs
# stand in: one never hands a frame on, as a camera whose setup outlasts the
# wait, and one gives two seconds of frames and stops, as one that dies mid
# call. The tests are relay-waits-vm-test.py, which pytest runs inside the
# driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  relay = pkgs.callPackage ../../../modules/v4l2loopback/relay.nix { };
  plugins = pkgs.lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" [
    pkgs.gst_all_1.gstreamer.out
    pkgs.gst_all_1.gst-plugins-base
    pkgs.gst_all_1.gst-plugins-good
  ];
  caps = "video/x-raw,format=NV12,width=320,height=240,framerate=30/1";
  # Each input's frames, before anything is done to them.
  frames = "videotestsrc is-live=true";
  inputs = {
    # A valve shut for good passes no frame, as a camera still setting up.
    never = "${frames} ! valve drop=true ! ${caps}";
    # Sixty frames at thirty a second, then nothing, though the pipeline
    # stays up, as a camera that stops mid call.
    stops = "${frames} num-buffers=60 ! ${caps}";
  };
  # A relay of each input, into a loopback of its own, as the shared cameras'
  # services run theirs, though left stopped until a test starts it, and
  # never started again by systemd, so a relay that ends stays ended.
  relayOf =
    ctl: name: input:
    let
      device = "/run/relay-${name}/device";
      output = pkgs.lib.concatStringsSep " ! " [
        "appsrc name=appsrc caps=${caps}"
        "queue leaky=downstream max-size-buffers=3"
        "videoconvert"
        "v4l2sink name=v4l2sink device=$(cat ${device}) sync=false"
      ];
    in
    {
      wants = [ "modprobe@v4l2loopback.service" ];
      after = [ "modprobe@v4l2loopback.service" ];
      environment.GST_PLUGIN_PATH = plugins;
      serviceConfig.RuntimeDirectory = "relay-${name}";
      preStart = "${ctl} add -n 'Relayed ${name}' -b 4 -x 1 > ${device}";
      script = ''
        exec ${pkgs.lib.getExe relay} -i ${pkgs.lib.escapeShellArg input} -o "${output}"
      '';
      postStop = ''${ctl} delete "$(cat ${device})" || true'';
    };
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "relay-waits";

  nodes.machine =
    { config, ... }:
    {
      imports = [ self.nixosModules.v4l2loopback ];
      # The loopback's driver, which the module adds only for a camera it
      # names holders for, and none made at its load, as the module has it.
      boot.extraModulePackages = [ config.jitsusama.v4l2loopback.package ];
      boot.extraModprobeConfig = "options v4l2loopback devices=0";
      systemd.services =
        let
          ctl = "${config.jitsusama.v4l2loopback.package.bin}/bin/v4l2loopback-ctl";
        in
        pkgs.lib.mapAttrs' (
          name: input: pkgs.lib.nameValuePair "relay-${name}" (relayOf ctl name input)
        ) inputs;
      # v4l2-ctl is the client reading each loopback.
      environment.systemPackages = [ pkgs.v4l-utils ];
    };

  tests = ./relay-waits-vm-test.py;
}
