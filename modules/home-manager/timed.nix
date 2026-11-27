# Which services an account's timers run, read one way: home.nix bounds
# and keeps each, and any check of a timed run finds the same ones, so the
# two can't disagree on what a timer runs or what the account writes.
{ lib }:
rec {
  # The unit file a service name is written as: a template's instance,
  # such as backup@home, is written as its template, backup@. An instance
  # the account writes as a unit of its own is still read as its template:
  # reading the account's services to tell would make their names depend
  # on themselves, which Nix can't evaluate. No timer in core runs one.
  written =
    name:
    let
      parts = lib.splitString "@" name;
    in
    if lib.length parts == 2 then "${lib.head parts}@" else name;

  # The service each of `timers` runs: the one its Timer section names, or
  # the one of its own name, as systemd has it, less those `unwritten`
  # says the account doesn't write.
  timed =
    {
      timers,
      unwritten ? [ ],
    }:
    lib.subtractLists unwritten (
      lib.unique (
        map (name: written (lib.removeSuffix ".service" name)) (
          lib.filter (lib.hasSuffix ".service") (
            lib.mapAttrsToList (name: timer: timer.Timer.Unit or "${name}.service") timers
          )
        )
      )
    );

  # Whether the account leaves service `name` of `services` unwritten.
  # home-manager gives every service an empty ExecStart, so an empty one,
  # or none, or no service at all, is one the account doesn't write.
  bare =
    services: name:
    lib.elem (services.${name}.Service.ExecStart or [ ]) [
      [ ]
      ""
      null
    ];
}
