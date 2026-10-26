# Programs that come with a machine but that nobody starts from the
# launcher stay out of it: each module that brings one names its entry
# here, beside the reason it brings it, and the entry is masked
# (entry.nix). NixOS itself brings its manual's, which opens the manual
# in a browser; the manual is read on the web or through man
# configuration.nix, which stay.
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
    example = [ "virt-manager" ];
    description = ''
      The desktop entries, by name without .desktop, of programs the machine
      installs that nobody starts from the launcher. Each is masked, so
      every list of programs leaves it out, while the program still runs
      from a terminal.
    '';
  };

  config = {
    jitsusama.unlisted = [ "nixos-manual" ];
    environment.systemPackages = map (pkgs.callPackage ./entry.nix { }) config.jitsusama.unlisted;
  };
}
