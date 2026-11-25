# The warnings a machine's lend (home.nix's `lend`) gets for a models budget
# or a machine that mycelium's lender would refuse every model through. It
# changes only when what the lender refuses a model for does, apart from
# the lend's shape, which lend.nix holds.
#
# Warned of rather than refused, so a machine lending models before its
# lend caps their spend still evaluates; mycelium refuses each such model
# lend 403, so no model is lent without a cap, though the cap counts only
# what lend.nix's usd_a_day says it does. A models budget is warned of
# whether a machine lends through it yet or not, since it refuses every
# model from the moment one does.
{
  config,
  lib,
  options,
}:
let
  cfg = config.jitsusama.mycelium;
  models = lib.filterAttrs (_: budget: budget.kind == "models") cfg.lend.budgets;
  # The budgets this account's spend-watch answers for, where it runs
  # here: with none declared, its one budget is `default`. A name it
  # doesn't answer for is refused as one naming none is.
  watched =
    let
      declared = lib.attrNames config.jitsusama.spend-watch.budgets;
    in
    if !(options.jitsusama ? spend-watch) then
      null
    else if declared == [ ] then
      [ "default" ]
    else
      declared;
  unknownTo =
    budget:
    let
      named = budget.spend_watch;
    in
    watched != null && named != null && !lib.elem named watched;
  lendsModels = vm: lib.any (budget: models ? ${budget}) ([ vm.budget ] ++ vm.also);
  missing =
    budget:
    lib.optional (budget.prices == null || budget.prices == { }) "prices"
    ++ lib.optional (budget.spend_watch == null) "spend_watch";
in
{
  warnings =
    lib.mapAttrsToList (
      name: budget:
      "jitsusama.mycelium.lend.budgets.${name} names no ${lib.concatStringsSep " and no " (missing budget)}, so mycelium refuses every model lent through it."
    ) (lib.filterAttrs (_: budget: missing budget != [ ]) models)
    ++ lib.mapAttrsToList (
      name: budget:
      "jitsusama.mycelium.lend.budgets.${name} names ${budget.spend_watch} in spend_watch, which this account's spend-watch doesn't declare, so mycelium refuses every model lent through it."
    ) (lib.filterAttrs (_: unknownTo) models)
    ++ map (
      vm:
      "jitsusama.mycelium.lend.vms' ${vm.prefix} names no usd_a_day, so mycelium lends its machines no model."
    ) (lib.filter (vm: lendsModels vm && vm.usd_a_day == null) cfg.lend.vms);
}
