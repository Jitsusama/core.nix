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

  # A time span as systemd's parse_time reads one: numbers, each with a
  # unit it knows or none, and spaces or tabs between. A number is whole,
  # with a plus before it or not, and may have a stop and digits after;
  # or it is a stop and digits alone. A number with no unit must be
  # followed by a space or tab, or end the span, so "1.5.0h" and "10+5"
  # are refused, and "1.h" and "+.5h" too, as systemd refuses each.
  span =
    let
      number = "(\\+?[0-9]+(\\.[0-9]+)?|\\.[0-9]+)";
      united = "${number}[[:blank:]]*(${units})";
    in
    "[[:blank:]]*((${united})[[:blank:]]*|${number}[[:blank:]]+)*((${united})|${number})[[:blank:]]*";

  # The units systemd's time spans take.
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

  # A key's value as systemd reads it: home-manager writes a list as one
  # line each, and systemd skips a line it can't read, so the last line it
  # can read wins, or, when it can read none, the last is the one said; a
  # string is read trimmed.
  said =
    value:
    let
      lines = map (line: if builtins.isString line then lib.trim line else line) (lib.toList value);
      kept = lib.filter readsAs lines;
    in
    if lines == [ ] then
      null
    else if !builtins.isList value then
      lib.head lines
    else if kept != [ ] then
      lib.last kept
    else
      lib.last lines;

  # Whether systemd reads one line as a timeout: a number not below zero,
  # a span, or infinity.
  readsAs =
    v:
    ((builtins.isInt v || builtins.isFloat v) && v >= 0)
    || (builtins.isString v && (builtins.match span v != null || v == "infinity"));

  # A timeout systemd reads: one whose line it reads. One it can't read
  # it drops, as if it weren't there.
  readable = value: readsAs (said value);

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
      # An empty list writes no line, so it is no key given.
      given = lib.filter (key: service ? ${key} && service.${key} != [ ]) [
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
