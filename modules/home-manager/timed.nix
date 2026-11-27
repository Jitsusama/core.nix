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
  # home-manager's systemd-reads-each-span-as-core-does holds it to the
  # systemd nixpkgs ships, so a systemd reading otherwise is caught when
  # it comes rather than by a run killed or left unbounded.
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

  # The lines of a list systemd drops after the one it reads, in order: a
  # later line is how a list overrides an earlier one, so a line dropped
  # for its spelling leaves the earlier bound standing where none sees
  # it. None when it reads no line, since then the key itself is unread.
  late =
    value:
    let
      lines = map (line: if builtins.isString line then lib.trim line else line) (lib.toList value);
      after =
        lib.foldr
          (
            line: found:
            if found.read then
              found
            else if readsAs line then
              found // { read = true; }
            else
              found // { lines = [ line ] ++ found.lines; }
          )
          {
            read = false;
            lines = [ ];
          }
          lines;
    in
    if after.read then after.lines else [ ];

  # One line of a key as home-manager writes it: a number as Nix's
  # toString gives it, so a float keeps six decimals and 1.0e-7 is
  # written "0.000000"; anything else as it is.
  inFile = v: if builtins.isInt v || builtins.isFloat v then toString v else v;

  # Whether systemd reads one line as a timeout: a span, as written, or
  # infinity.
  readsAs =
    v:
    let
      line = inFile v;
    in
    builtins.isString line && (builtins.match span line != null || line == "infinity");

  # How many microseconds each unit is, as systemd counts them, a number
  # with none being seconds.
  micros =
    let
      each = units: count: lib.genAttrs units (_: count);
    in
    each [ "usec" "us" "µs" "μs" ] 1
    // each [ "msec" "ms" ] 1000
    // each [ "seconds" "second" "sec" "s" ] 1000000
    // each [ "minutes" "minute" "min" "m" ] 60000000
    // each [ "hours" "hour" "hr" "h" ] 3600000000
    // each [ "days" "day" "d" ] 86400000000
    // each [ "weeks" "week" "w" ] 604800000000
    // each [ "months" "month" "M" ] 2629800000000
    // each [ "years" "year" "y" ] 31557600000000;

  # Whether a span systemd reads comes to a microsecond or more. systemd
  # counts each number's digits after the stop down to the microsecond and
  # drops the rest, so "0.5us" and "0.0000001s" are zero, which bounds
  # nothing: a digit counts when its place, the unit divided by ten once
  # for each digit up to it, is still a microsecond or more.
  lasts =
    line:
    let
      parts = lib.filter builtins.isList (
        builtins.split "([0-9.]+)[[:blank:]]*(${units})?" (lib.replaceStrings [ "+" ] [ "" ] line)
      );
      counts =
        part:
        let
          number = lib.splitString "." (lib.head part);
          unit = lib.elemAt part 1;
          each = micros.${if unit == null then "s" else unit};
          after = if lib.length number > 1 then lib.elemAt number 1 else "";
          zeros = builtins.match "(0*)[1-9].*" after;
          place = lib.foldl' (left: _: left / 10) each (lib.range 0 (lib.stringLength (lib.head zeros)));
        in
        builtins.match ".*[1-9].*" (lib.head number) != null || (zeros != null && place > 0);
    in
    lib.any counts parts;

  # A timeout systemd reads: one whose line it reads. One it can't read
  # it drops, as if it weren't there.
  readable = value: readsAs (said value);

  # Whether a timeout ends a hang: a span, as written, of a microsecond or
  # more. Zero, a span under a microsecond, and infinity end nothing.
  ends =
    value:
    let
      line = inFile (said value);
    in
    builtins.isString line && builtins.match span line != null && lasts line;

  # The start timeout a service's `Service` section sets, as systemd reads
  # it: `given`, the keys it says; `read`, those systemd can read;
  # `dropped`, those of `read` with a line after the one systemd reads
  # that it drops; and `value`, the one that bounds the start, or null
  # for none. Order counts only when both are readable: home-manager
  # writes TimeoutSec before TimeoutStartSec, and the later line wins, so
  # TimeoutStartSec does.
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
      dropped = lib.filter (key: late service.${key} != [ ]) read;
      value = if read == [ ] then null else said service.${lib.head read};
    };
}
