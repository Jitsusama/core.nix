# The check that every daemon an account runs keeps the convention
# docs/daemons.md sets, for core and for any repository built on it, as
# `core.lib.daemonsAgree pkgs`: daemons-agree.py, beside this, says what each
# point holds.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it, whose crates build the
  # daemons.
  src,
  # An evaluated home-manager account, whose units run the daemons.
  home,
  # The points of the convention each daemon broke before the rule, by its
  # name, while it comes to agree. The check fails on a daemon breaking a
  # point it doesn't wait on, and on one waiting on a point it keeps.
  waiting ? { },
  name ? "daemons-agree",
}:
let
  inherit (home.systemd.user) sockets services;
  # A unit's words as written, without what building them would need, since
  # the check reads a command line and builds none of the programs in it.
  said = value: builtins.unsafeDiscardStringContext (toString value);
  # The command a service runs, the last of its lines, as Home Manager keeps
  # an ExecStart a list when one empties the line before.
  command =
    service:
    said (
      lib.findFirst (line: line != "") "" (
        lib.reverseList (lib.toList (service.Service.ExecStart or [ ]))
      )
    );
  units = {
    sockets = lib.mapAttrs (unit: socket: {
      listens = map said (lib.toList (socket.Socket.ListenStream or [ ]));
      service = lib.removeSuffix ".service" (socket.Socket.Service or "${unit}.service");
    }) sockets;
    services = lib.mapAttrs (_: service: { command = command service; }) services;
  };
in
runCommandLocal name { } ''
  ${lib.getExe python3} ${./daemons-agree.py} ${src} \
    ${writeText "units.json" (builtins.toJSON units)} \
    ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
