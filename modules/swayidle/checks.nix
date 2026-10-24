# The checks of swayidle, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  ...
}:
{
  # The screens go off through the machine's own niri, the one with core's
  # patches, so the account doesn't carry a second one just to ask.
  one-niri =
    let
      niri = lib.getExe graphical.config.programs.niri.package;
      commands = map (timeout: timeout.command) (account graphical).services.swayidle.timeouts;
    in
    holds "one-niri" "swayidle calls a niri other than the machine's." (
      lib.any (lib.hasInfix "/bin/niri ") commands
      && lib.all (command: !lib.hasInfix "/bin/niri " command || lib.hasPrefix niri command) commands
    );
}
