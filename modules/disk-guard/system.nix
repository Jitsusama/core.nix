# What the machine's disk guard is given the same way on NixOS and on a Mac:
# its config, which nixos.nix and darwin.nix each fill with that system's
# sources, and the file the guard reads it from. The file is rendered as the
# config is, its null sources dropped, and the build refuses one the guard's
# check-config refuses.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  json = pkgs.formats.json { };
  settings = config.jitsusama.disk-guard.settings;

  rendered = settings // {
    version = 1;
    scope = "system";
    sources = lib.filterAttrs (_: source: source != null) settings.sources;
  };
in
{
  options.jitsusama.disk-guard = {
    settings = lib.mkOption {
      type = lib.types.submodule { freeformType = json.type; };
      default = { };
      description = ''
        The machine's guard's config: its bands, its sources, and its policy
        for Nix's garbage, generations and linking. A source set to null is
        dropped.
      '';
    };
    configFile = lib.mkOption {
      type = lib.types.package;
      internal = true;
      readOnly = true;
      description = "The config as the guard reads it, refused at build by the guard itself.";
    };
  };

  config.jitsusama.disk-guard.configFile =
    pkgs.runCommandLocal "disk-guard-system.json"
      {
        text = builtins.toJSON rendered;
        passAsFile = [ "text" ];
      }
      ''
        ${lib.getExe package} check-config "$textPath"
        cp "$textPath" $out
      '';
}
