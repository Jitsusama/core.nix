# What a machine declares of itself, in mycelium's words, as the JSON the
# node and the machine's commands read (home.nix writes it twice: in the
# store for the node, and in the account's config folder for the rest).
{ lib }:
cfg:
let
  # Only the fences a machine says run here, in mycelium's shape. An
  # agent's `mycelium run` in a fenced or strict one starts through the
  # fence's launcher, Landlock's on Linux and sandbox-exec's on a Mac; one
  # unfenced takes none, as no agent asks as unfenced.
  fenced = lib.mapAttrs (kind: doors: doors // runsIn kind) (
    lib.filterAttrs (_: doors: doors != null) cfg.declares.fence
  );
  runsIn =
    kind:
    lib.optionalAttrs (cfg.fenceRuns != null && kind != "none") {
      runs = cfg.fenceRuns;
    };
in
builtins.toJSON (
  {
    inherit (cfg.declares) reach outlives taints;
    probes = lib.mapAttrs (
      _: probe: if builtins.isList probe then { run = probe; } else { connect = probe; }
    ) cfg.declares.probes;
    costs_money = cfg.declares.costsMoney;
  }
  // lib.optionalAttrs (cfg.agents != { }) { inherit (cfg) agents; }
  // lib.optionalAttrs (cfg.declares.within != null) { inherit (cfg.declares) within; }
  // lib.optionalAttrs (cfg.declares.hosts != [ ]) { inherit (cfg.declares) hosts; }
  // lib.optionalAttrs (fenced != { }) { fence = fenced; }
)
