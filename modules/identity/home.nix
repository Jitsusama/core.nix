# Who Joel is on a machine, for the tools that attribute his work to him.
# The name is the same everywhere; each layer sets the email it works under.
{ lib, ... }:
{
  options.jitsusama.identity = {
    name = lib.mkOption {
      type = lib.types.str;
      default = "Joel Gerber";
      description = "The name Joel's work is attributed to.";
    };
    email = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "joel@grrbrr.ca";
      description = "The email address Joel's work is attributed to, or null to leave it unset.";
    };
  };
}
