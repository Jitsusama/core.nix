# Which services an account's timers run, and the timeout each sets, read
# one way: home.nix bounds and keeps each, and any check of a timed run
# finds the same ones, so the two can't disagree on what a timer runs, what
# the account writes, or which timeout systemd reads.
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
  # or none, or no service at all, is one the account doesn't write, as is
  # one whose every line is empty or blank, which systemd reads as none.
  bare =
    services: name:
    lib.all (line: line == null || lib.trim (toString line) == "") (
      lib.toList (services.${name}.Service.ExecStart or [ ])
    );

  # A time span as systemd reads one: numbers, each with a unit it knows
  # or none.
  span =
    let
      units = lib.concatStringsSep "|" [
        "usec"
        "us"
        "msec"
        "ms"
        "seconds"
        "second"
        "sec"
        "s"
        "minutes"
        "minute"
        "min"
        "m"
        "hours"
        "hour"
        "hr"
        "h"
        "days"
        "day"
        "d"
        "weeks"
        "week"
        "w"
        "months"
        "month"
        "M"
        "years"
        "year"
        "y"
      ];
    in
    "( *[0-9]+(\\.[0-9]+)? *(${units})?)+ *";

  # A timeout systemd reads: a whole number, a span, or infinity. One it
  # can't read it drops, as if it weren't there.
  readable =
    value:
    (builtins.isInt value && value >= 0)
    || (builtins.isString value && (builtins.match span value != null || value == "infinity"));

  # Whether a timeout ends a hang: a whole number above zero, or a span
  # holding a digit other than zero. Zero and infinity end nothing.
  ends =
    value:
    (builtins.isInt value && value > 0)
    || (
      builtins.isString value
      && builtins.match span value != null
      && builtins.match ".*[1-9].*" value != null
    );

  # The start timeout a service's `Service` section sets, as systemd reads
  # it: `given`, the keys it says; `read`, those systemd can read; and
  # `value`, the one that bounds the start, or null for none. Order counts
  # only when both are readable: home-manager writes TimeoutSec before
  # TimeoutStartSec, and the later line wins, so TimeoutStartSec does.
  timeout =
    service:
    let
      given = lib.filter (key: service ? ${key}) [
        "TimeoutStartSec"
        "TimeoutSec"
      ];
      read = lib.filter (key: readable service.${key}) given;
    in
    {
      inherit given read;
      value = if read == [ ] then null else service.${lib.head read};
    };
}
