# A fixed-output build that writes its case's name and then each variable it
# names, as the builder found them, so it builds only if the daemon handed it
# exactly the values `expect` says. Each case writes its own name first, so no
# two share an output and none is found already built.
{
  sh,
  case,
  vars,
  expect,
}:
let
  names = builtins.filter builtins.isString (builtins.split " " vars);
  each = builtins.concatStringsSep "" (map (name: '':"''$${name}"'') names);
in
builtins.derivation {
  name = case;
  system = builtins.currentSystem;
  builder = sh;
  args = [
    "-c"
    ''printf %s "${case}"${each} > "$out"''
  ];
  impureEnvVars = names;
  outputHashMode = "flat";
  outputHashAlgo = "sha256";
  outputHash = builtins.hashString "sha256" expect;
}
