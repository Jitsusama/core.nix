# What a machine declares of itself, in mycelium's words, as the JSON the
# node and the machine's commands read (home.nix writes it twice: in the
# store for the node, and in the account's config folder for the rest).
{ lib, callIsOn }:
cfg:
let
  # Only the fences a machine says run here, in mycelium's shape. An
  # agent's `mycelium run` in a fenced or strict one starts through the
  # fence's launcher, Landlock's on Linux and sandbox-exec's on a Mac; one
  # unfenced takes none, as no agent asks as unfenced. Doors accepted for
  # other machines are said only once some machine is named, as an empty
  # allowance means what none does.
  fenced = lib.mapAttrs (
    kind: doors:
    removeAttrs doors [ "accepts" ]
    // lib.optionalAttrs (doors.accepts != { }) { inherit (doors) accepts; }
    // runsIn kind
  ) (lib.filterAttrs (_: doors: doors != null) cfg.declares.fence);
  runsIn =
    kind:
    lib.optionalAttrs (cfg.fenceRuns != null && kind != "none") {
      runs = cfg.fenceRuns;
    };
in
builtins.toJSON (
  {
    inherit (cfg.declares) reach outlives taints;
    # Beside the networks it names, the node of every machine that declares
    # itself says whether a call is on there, as `reached.call`: a-call-is-on only asks the sound
    # server whether a program records (Core Audio's process list on a
    # Mac), opens no device and needs no microphone grant, and the node
    # publishes its yes or no alone. Another of the person's machines
    # brakes its builds by it (modules/slices, jobs-sorter's elsewhere.rs),
    # and its a-call-is-on counts it a call there too, so the probe asks
    # about this machine alone (`--here`): asking its peers as well, two
    # machines would each hold up the other's call for ever.
    probes =
      lib.mapAttrs (
        _: probe: if builtins.isList probe then { run = probe; } else { connect = probe; }
      ) cfg.declares.probes
      // {
        call.run = [
          callIsOn
          "--here"
        ];
      };
    costs_money = cfg.declares.costsMoney;
  }
  // lib.optionalAttrs (cfg.agents != { }) { inherit (cfg) agents; }
  // lib.optionalAttrs (cfg.declares.within != null) { inherit (cfg.declares) within; }
  // lib.optionalAttrs (cfg.declares.hosts != [ ]) { inherit (cfg.declares) hosts; }
  // lib.optionalAttrs (fenced != { }) { fence = fenced; }
)
