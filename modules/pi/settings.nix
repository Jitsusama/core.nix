# The program that puts the settings declared for pi (jitsusama.pi.settings)
# in the settings file pi keeps. pi writes that file itself, as /settings
# changes it and as it records what it has shown, so the program keeps every
# key pi wrote, gives the declared ones their values, and leaves a file it
# can't read as JSON for a person to look at rather than replacing it.
{ pkgs, settings }:
pkgs.writeShellApplication {
  name = "pi-settings";
  runtimeInputs = [
    pkgs.coreutils
    pkgs.jq
  ];
  runtimeEnv.DECLARED = pkgs.writeText "pi-settings.json" (builtins.toJSON settings);
  text = builtins.readFile ./settings.sh;
}
