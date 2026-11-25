# Which of the user services are sound services, which realtime.rules grants
# rtkit's real time by name (nixos.nix gives it them): PipeWire's own three,
# each that runs a PipeWire of its own, as a tuning chain does, and each
# PipeWire wants that sits in session.slice beside it, as noise-cancelled
# does. A watcher PipeWire wants in app.slice, such as the mic-mute light,
# keeps rtkit's own answer. Apart from nixos.nix so its checks can give it
# services of their own.
{ lib }:
{
  # The pipewire program, as a store path.
  pipewire,
  # NixOS's systemd.user.services, every account's.
  system,
  # Each account's home-manager systemd.user.services, by its login name.
  homes,
}:
let
  # A list is several commands, one ExecStart= line each, as NixOS and
  # home-manager both write it, and an empty one resets those before it, so
  # only those after the last empty one run. Each is read without the
  # prefixes systemd takes for how it runs one (-, @, :, +, !), and as the
  # program alone or with a space after, bare or quoted as NixOS's
  # escapeSystemdExecArgs writes it, so pipewire-pulse, or another program
  # whose path begins the same, is none. A command marked | goes to the
  # account's shell, which may run more than PipeWire, so it's none, as is
  # a chain a NixOS unit declares through script, which runs a script of its
  # own; each is granted only if PipeWire wants it beside it.
  runs =
    start:
    lib.foldl' (after: command: if command == "" then [ ] else after ++ [ command ]) [ ] (
      map toString (lib.toList start)
    );
  isPipewire = program: bare: bare == program || lib.hasPrefix "${program} " bare;
  runsPipewire =
    start:
    lib.any (
      command:
      let
        bare = lib.head (builtins.match "[-@:+!]*(.*)" command);
      in
      isPipewire pipewire bare || isPipewire (builtins.toJSON pipewire) bare
    ) (runs start);
  # A unit may name what wants it as a list or as one name.
  wanted = units: lib.elem "pipewire.service" (lib.toList units);
  sound =
    start: wantedBy: slice:
    runsPipewire start || (wanted wantedBy && slice == "session.slice");
  named = isSound: services: lib.attrNames (lib.filterAttrs (_: isSound) services);
in
{
  # Granted in any account's manager. A NixOS unit switched off runs nowhere.
  system = lib.unique (
    [
      "pipewire"
      "pipewire-pulse"
      "wireplumber"
    ]
    ++ named (
      service:
      (service.enable or true)
      && sound (service.serviceConfig.ExecStart or "") (service.wantedBy or [ ]) (
        service.serviceConfig.Slice or ""
      )
    ) system
  );
  # Granted only in the manager of the account whose service it is.
  homes = lib.mapAttrs (
    _:
    named (
      service:
      sound (service.Service.ExecStart or "") (service.Install.WantedBy or [ ]) (
        service.Service.Slice or ""
      )
    )
  ) homes;
}
