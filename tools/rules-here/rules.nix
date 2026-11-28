# What rules-here runs: each rule tests/rules/checks.nix holds whose check is
# a script over the source tree, as the command its check would run, and each
# rule it can't run that way, with why. rules-here.sh evaluates this and runs
# what it says; checks.nix beside it evaluates it over stand-in rules.
#
# A rule is run here when its check is given only data beside the tree (a
# waiting list, a name, files to include) and its own file asks only for
# programs from nixpkgs, `lib`, `runCommandLocal` and `writeText`. It's then
# the check's own command, its tree read from "$RULES_TREE" when it runs,
# each waiting list written into the store as its check writes it, and its
# programs those the checks build with, so it passes and fails as its check
# would. A rule whose check is given a machine, a package or treefmt's
# settings, or whose file asks for `callPackage` to build something, is named
# as not run, with why.
{
  # The tree the rules are read from: their checks file and its waiting
  # lists, and flake.nix for core's own rules.
  root,
  # The checks file holding the rules.
  rules ? root + "/tests/rules/checks.nix",
  # nixpkgs at the pin the checks build with, from the tree's flake.lock.
  pkgs ?
    let
      lock = builtins.fromJSON (builtins.readFile (root + "/flake.lock"));
      nixpkgs = builtins.fetchTree lock.nodes.${lock.nodes.${lock.root}.inputs.nixpkgs}.locked;
    in
    import nixpkgs { system = builtins.currentSystem; },
  # core's own outputs, whose `lib` holds each rule's check, from the tree's
  # flake.nix. Only `lib` is read, so its inputs are never needed.
  self ?
    let
      flake = import (root + "/flake.nix");
      outputs = flake.outputs (
        builtins.mapAttrs (
          input: _: if input == "self" then outputs else throw "rules-here needs no ${input}"
        ) (builtins.functionArgs flake.outputs)
      );
    in
    outputs,
}:
let
  inherit (pkgs) lib;

  # Each rule's check as it's called: its file, and what it's given. A check
  # made any other way throws when it's read, and is named as not run.
  called = import rules {
    inherit self;
    pkgs = pkgs // {
      callPackage = file: given: arguments: { inherit file given arguments; };
    };
    onLinux = lib.optionalAttrs (pkgs.stdenv.hostPlatform.system == "x86_64-linux");
    exampleLaptop = throw "it reads an evaluated machine";
    treefmtSettings = throw "it reads treefmt's settings";
  };

  # Whether a value is data alone: no function, package or machine in it.
  isData =
    value:
    let
      type = builtins.typeOf value;
    in
    if type == "set" then
      !lib.isDerivation value && builtins.all isData (builtins.attrValues value)
    else if type == "list" then
      builtins.all isData value
    else
      type != "lambda";

  # What a check's own file is given beside its own words: its programs come
  # from nixpkgs, and what would build is stood in for by what runs here.
  standIns = {
    # The command alone, with the programs it's given on PATH.
    runCommandLocal = _name: env: command: { inherit command env; };
    # A file written into the store as the tree is read, as writeText's is,
    # with no build.
    writeText = builtins.toFile;
  };

  # Why a check's file can't be run here, or null when it can.
  whyNot =
    file:
    let
      asks = builtins.attrNames (builtins.functionArgs (import file));
      builds = builtins.filter (
        name: name != "lib" && !(standIns ? ${name}) && !lib.isDerivation (pkgs.${name} or null)
      ) asks;
    in
    if builds == [ ] then null else "its check builds with ${lib.concatStringsSep " and " builds}";

  # One rule, as rules-here runs it, or why it can't.
  plan =
    name: check:
    let
      read = builtins.tryEval (builtins.seq check.file check);
      given = removeAttrs read.value.arguments [
        "src"
        "name"
      ];
      dataOnly = builtins.tryEval (isData given);
      made = pkgs.callPackage read.value.file (standIns // read.value.given) (
        read.value.arguments // { src = ''"$RULES_TREE"''; }
      );
      tools = made.env.nativeBuildInputs or [ ];
    in
    if !read.success || !(read.value ? file) then
      {
        inherit name;
        why = "its check is no script over the tree";
      }
    else if !dataOnly.success || !dataOnly.value then
      {
        inherit name;
        why = "its check is given something built beside the tree";
      }
    else if whyNot read.value.file != null then
      {
        inherit name;
        why = whyNot read.value.file;
      }
    else
      {
        inherit name;
        inherit (made) command;
        path = lib.makeBinPath (tools ++ pkgs.stdenv.initialPath);
      };

  planned = lib.mapAttrsToList plan called;
  runs = builtins.filter (rule: rule ? command) planned;
  notRun = builtins.filter (rule: rule ? why) planned;
  # Each rule not run, then why, as rules-here.sh reads them in pairs.
  notRunPairs = lib.concatMap (rule: [
    rule.name
    rule.why
  ]) notRun;

  # Every store path the commands name that comes from a build, to be
  # fetched before they run: the programs, and none of the tree's files.
  needs = lib.unique (
    lib.concatMap (
      rule:
      lib.mapAttrsToList (drv: held: "${drv}^${lib.concatStringsSep "," held.outputs}") (
        lib.filterAttrs (_: held: held ? outputs) (
          builtins.getContext (rule.command + rule.path + pkgs.runtimeShell)
        )
      )
    ) runs
  );

  words = values: lib.concatMapStringsSep " " lib.escapeShellArg values;
in
# Bash that sets the shell the commands run in and five arrays, which
# rules-here.sh sources: the rules to run with their commands and PATHs,
# those not run with why, and what to fetch first. It keeps the programs it
# names as its context, so a check that writes it into the store holds them.
''
  shell=${lib.escapeShellArg pkgs.runtimeShell}
  run_names=(${words (map (rule: rule.name) runs)})
  run_commands=(${words (map (rule: rule.command) runs)})
  run_paths=(${words (map (rule: rule.path) runs)})
  not_run=(${words notRunPairs})
  needs=(${words needs})
''
