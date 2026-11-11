# A build asking for a range of user ids, as a check whose machines are
# containers does, that writes how many ids it was given.
derivation {
  name = "uid-range";
  system = builtins.currentSystem;
  builder = "/bin/sh";
  args = [
    "-c"
    ''read -r inside outside count < /proc/self/uid_map; echo "$count" > "$out"''
  ];
  requiredSystemFeatures = [ "uid-range" ];
}
