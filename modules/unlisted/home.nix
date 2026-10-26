# Programs an account installs that nobody starts from the launcher stay
# out of it: each module that brings one names its entry here, beside the
# reason it brings it, and the entry is masked in the account's profile
# (entry.nix), which the launcher reads before the machine's.
{
  config,
  lib,
  pkgs,
  ...
}:
{
  options.jitsusama.unlisted = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [ "umpv" ];
    description = ''
      The desktop entries, by name without .desktop, of programs the account
      installs that nobody starts from the launcher. Each is masked, so
      every list of programs leaves it out, while the program still runs
      from a terminal.
    '';
  };

  config.home.packages = map (pkgs.callPackage ./entry.nix { }) config.jitsusama.unlisted;
}
