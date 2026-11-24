# The checks of nixpkgs' settings, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  bareDarwin,
  bareNixos,
  holds,
  ...
}:
let
  # Whether a machine's nixpkgs lets a package evaluate: its derivation's
  # path is refused for an unfree package no list names.
  allows =
    machine: name:
    let
      inherit (machine.config._module.args) pkgs;
    in
    (builtins.tryEval pkgs.${name}.drvPath).success;

  # A name a machine adds for itself, of a package core names nowhere.
  naming.nixpkgs.config.allowUnfreePackages = [ "terraform" ];

  # A machine's own name joins core's rather than replacing them, and an
  # unfree package nobody names stays refused.
  joins =
    bare:
    let
      plain = bare [ ];
      named = bare [ naming ];
    in
    allows plain "claude-code"
    && !allows plain "terraform"
    && allows named "claude-code"
    && allows named "terraform";
in
{
  a-machines-unfree-names-join-cores-on-nixos =
    holds "a-machines-unfree-names-join-cores-on-nixos"
      "On NixOS, a machine's unfree names don't join core's, or one nobody names is let through."
      (joins bareNixos);

  a-machines-unfree-names-join-cores-on-darwin =
    holds "a-machines-unfree-names-join-cores-on-darwin"
      "On nix-darwin, a machine's unfree names don't join core's, or one nobody names is let through."
      (joins bareDarwin);
}
