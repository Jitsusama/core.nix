# The checks of nixpkgs' settings, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  bareDarwin,
  bareNixos,
  holds,
  lib,
  macWorkstation,
  workstation,
  ...
}:
let
  # Whether a machine's nixpkgs lets a package evaluate. Refusing an unfree
  # package no list names is a throw, which tryEval catches; any other
  # error ends the check's evaluation with its own message rather than
  # passing as a refusal.
  allows =
    machine: name:
    let
      inherit (machine.config._module.args) pkgs;
    in
    (builtins.tryEval pkgs.${name}.drvPath).success;

  # Whether nixpkgs still marks a package unfree, so that the checks below
  # fail, saying so, rather than passing for nothing once it doesn't.
  unfree = machine: name: machine.config._module.args.pkgs.${name}.meta.unfree;

  # A name a machine adds for itself, of a package core names nowhere.
  naming.nixpkgs.config.allowUnfreePackages = [ "terraform" ];

  # A machine's own name joins core's rather than replacing them, and a role
  # machine, with every module it brings, still refuses an unfree package
  # nobody names, so no module of core's allows every unfree package.
  joins =
    bare: role:
    let
      named = bare [ naming ];
    in
    assert lib.assertMsg (
      unfree role "claude-code" && unfree role "terraform"
    ) "claude-code or terraform is no longer unfree, so these checks prove nothing.";
    allows role "claude-code"
    && !allows role "terraform"
    && allows named "claude-code"
    && allows named "terraform";
in
{
  a-machines-unfree-names-join-cores-on-nixos =
    holds "a-machines-unfree-names-join-cores-on-nixos"
      "On NixOS, a machine's unfree names don't join core's, or one nobody names is let through."
      (joins bareNixos workstation);

  a-machines-unfree-names-join-cores-on-darwin =
    holds "a-machines-unfree-names-join-cores-on-darwin"
      "On nix-darwin, a Mac's unfree names don't join core's, or one nobody names is let through."
      (joins bareDarwin macWorkstation);
}
