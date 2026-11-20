# The camera service's start for its loopback (loopback.sh), over the
# v4l2loopback-ctl given: the module's own on the machine, and a stand-in in
# its check.
{
  coreutils,
  ctl,
  writeShellApplication,
}:
writeShellApplication {
  name = "camera-loopback";
  runtimeInputs = [
    coreutils
    ctl
  ];
  text = builtins.readFile ./loopback.sh;
}
