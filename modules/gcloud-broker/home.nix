# gcloud-broker mints hour-long Google Cloud tokens for one service
# account, with the person's own gcloud login (src/lib.rs says why). A
# sandbox's fence leaves that login out of reach, so what asks is the
# program starting a sandboxed command, outside it, which hands the command
# the token alone, with the account's project.
#
# The account is a machine's to name, in jitsusama.gcloud-broker; until one
# is named nothing is installed, and commands get no token. The person's
# login may mint its tokens only once they're granted Token Creator on it.
#
# `gcloud-broker` on PATH asks as that account, for gcloud by hand too:
#   CLOUDSDK_AUTH_ACCESS_TOKEN=$(gcloud-broker access) \
#     CLOUDSDK_CORE_PROJECT=$(gcloud-broker project) gcloud ...
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.gcloud-broker;
  package = pkgs.callPackage ./package.nix { };
  asking = pkgs.writeShellScriptBin "gcloud-broker" ''
    exec ${lib.getExe package} "$@" --account ${lib.escapeShellArg cfg.serviceAccount}
  '';
in
{
  options.jitsusama.gcloud-broker.serviceAccount = lib.mkOption {
    type = lib.types.nullOr (lib.types.strMatching "[a-z0-9-]+@[a-z0-9-]+\\.iam\\.gserviceaccount\\.com");
    default = null;
    example = "agents@a-lab.iam.gserviceaccount.com";
    description = ''
      The service account whose tokens are minted: one a project made for
      agents, holding roles in that project alone, that the person's login
      may mint tokens for. Its project is where a sandboxed command's
      gcloud works.
    '';
  };

  config = lib.mkIf (cfg.serviceAccount != null) {
    home.packages = [ asking ];
  };
}
