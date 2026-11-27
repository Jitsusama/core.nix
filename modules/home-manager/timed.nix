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
  # one whose last line is empty or blank: systemd reads an empty line as
  # clearing the lines before it, so it reads none.
  bare =
    services: name:
    let
      lines = lib.toList (services.${name}.Service.ExecStart or [ ]);
      last = lib.last lines;
    in
    lines == [ ] || last == null || lib.trim (toString last) == "";

  # A time span as systemd reads one: numbers, each with a unit it knows
  # or none, a number as strtod reads it, with a sign or a stop at
  # either end, and spaces or tabs between.
  span =
    let
      units = lib.concatStringsSep "|" [
        "usec"
        "us"
        "µs"
        "μs"
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
    "([[:blank:]]*\\+?([0-9]+(\\.[0-9]*)?|\\.[0-9]+)[[:blank:]]*(${units})?)+[[:blank:]]*";

  # A key's value as systemd reads it: home-manager writes a list as one
  # line each, and the last line wins; a string is read trimmed.
  said =
    value:
    let
      last =
        if !builtins.isList value then
          value
        else if value == [ ] then
          null
        else
          lib.last value;
    in
    if builtins.isString last then lib.trim last else last;

  # A timeout systemd reads: a number not below zero, a span, or
  # infinity. One it can't read it drops, as if it weren't there.
  readable =
    value:
    let
      v = said value;
    in
    ((builtins.isInt v || builtins.isFloat v) && v >= 0)
    || (builtins.isString v && (builtins.match span v != null || v == "infinity"));

  # Whether a timeout ends a hang: a number above zero, or a span holding
  # a digit other than zero. Zero and infinity end nothing.
  ends =
    value:
    let
      v = said value;
      spanned = builtins.isString v && builtins.match span v != null;
    in
    ((builtins.isInt v || builtins.isFloat v) && v > 0)
    || (spanned && builtins.match ".*[1-9].*" v != null);

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
      value = if read == [ ] then null else said service.${lib.head read};
    };
}
