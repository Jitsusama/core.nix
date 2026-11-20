# The settings a machine declares for the harness, merged into the config
# file the harness keeps beside pi's at every switch. The harness writes that
# file itself, as its settings change, and a person edits it by hand, so it
# is merged as pi's own settings are (settings.nix) rather than linked.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.pi.harness) settings;
in
{
  options.jitsusama.pi.harness.settings = lib.mkOption {
    type = lib.types.submodule { freeformType = (pkgs.formats.json { }).type; };
    default = { };
    example = {
      sections.quest-workflow.budgetWhenCapped = "a-budget";
    };
    description = ''
      Settings merged into the harness's own config file at every switch, as
      `jitsusama.pi.settings` is into pi's: each wins over what the harness
      or a person wrote there, and every other key is kept. Nothing is
      written while none is declared, and a setting taken out of here stays
      in the file until it is taken out there too. The values sit in the
      store, readable by every account, so none may be a secret.
    '';
  };

  # A file left not JSON is said and skipped rather than stopping the rest
  # of the switch, since a person edits it by hand.
  config.home.activation.piHarnessTakesItsSettings = lib.mkIf (settings != { }) (
    lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      run ${
        lib.getExe (import ./settings.nix { inherit pkgs settings; })
      } ${lib.escapeShellArg "${config.xdg.configHome}/pi/agentic-harness.pi/config.json"} || :
    ''
  );
}
