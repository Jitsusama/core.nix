# What pi tells mycelium, where mycelium's module is loaded: the command an
# agent session starts with in each realm the machine joins, so whatever
# starts one, such as the launcher, runs the right pi there. It's plain pi in
# the home realm, and pi-claude in the realm Joel's own Claude plan works in,
# unless the machine is on that plan, where plain pi is all there is.
{
  config,
  lib,
  options,
  ...
}:
let
  inherit (config.jitsusama) mycelium pi;
  onClaude = pi.plan == "claude";
in
{
  config = lib.optionalAttrs (options ? jitsusama.mycelium.agents) {
    # A machine's own word wins over pi's.
    jitsusama.mycelium.agents = lib.mapAttrs (_: lib.mkDefault) (
      lib.optionalAttrs (mycelium.realm != null) { ${mycelium.realm} = "pi"; }
      // lib.optionalAttrs (!onClaude && pi.claude.realm != null) { ${pi.claude.realm} = "pi-claude"; }
    );
  };
}
