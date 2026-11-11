# The checks of gcloud-broker, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  workstation,
  ...
}:
onLinux {
  # gcloud-broker's tests run as it builds against a stand-in for gcloud: a
  # token minted as the account and printed alone, kept for most of its hour
  # where only the account reads it, an identity token per audience, a
  # refusal said, and an account given twice refused.
  gcloud-broker-mints-and-keeps-its-tokens = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in gcloud-broker, its tests included.
  gcloud-broker-passes-clippy = passesClippy "gcloud-broker-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # A workstation that names no account has no broker, and one that names
  # an account asks only as it.
  gcloud-broker-asks-as-the-named-account =
    let
      named = bareNixos [
        self.nixosModules.workstation
        {
          home-manager.sharedModules = [
            { jitsusama.gcloud-broker.serviceAccount = "agents@a-lab.iam.gserviceaccount.com"; }
          ];
        }
      ];
      broker = machine: lib.findFirst (p: p.name == "gcloud-broker") null (account machine).home.packages;
      # The script's own text, read from its derivation rather than built.
      asks = (broker named).text;
      # An address of only safe characters goes unquoted, so the check
      # quotes it the way the module does rather than guessing.
      given = lib.escapeShellArg "agents@a-lab.iam.gserviceaccount.com";
    in
    holds "gcloud-broker-asks-as-the-named-account"
      "gcloud-broker is installed where no account is named, or asks as another than the named one."
      (broker workstation == null && lib.hasInfix "\"$@\" --account ${given}\n" asks);
}
