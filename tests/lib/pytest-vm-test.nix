# A VM test whose tests are a file of pytest tests, which pytest runs inside
# the driver (vm_pytest.py): what every such test shares, so a test's own
# file says only its machines, its tests and what they need from Nix.
#
# Run by hand, the driver takes pytest's options from PYTEST_ADDOPTS, so
# PYTEST_ADDOPTS='-k <words>' runs only the tests whose names hold them, as
# run-subtests does by the check's name. Everything else given is the
# NixOS test's own, as runNixOSTest takes it.
{ pkgs }:
{
  # The file of tests.
  tests,
  # What the tests need from Nix: one JSON file, which they read parsed
  # through the `values` fixture, or a function of the test's nodes giving
  # it, for values read from the machines; or none.
  values ? null,
  # The Python packages the tests import beyond what VM tests share.
  extraPythonPackages ? _: [ ],
  # The modules among those packages pytest takes as plugins, by name, so
  # the fixtures they hold reach the tests without the tests importing
  # them: what several tests' files share.
  plugins ? [ ],
  ...
}@test:
let
  # pytest takes each module PYTEST_PLUGINS names as a plugin, as it would
  # one given with -p, its asserts rewritten too. It's set only when there
  # are plugins, and vm_pytest is left as it is, so a test without them
  # builds the same script and derivation as before: a change to
  # vm_pytest moves every pytest VM test.
  named = pkgs.lib.concatStringsSep "," plugins;
  taken = pkgs.lib.optionalString (
    plugins != [ ]
  ) ''import os; os.environ["PYTEST_PLUGINS"] = "${named}"; '';
  script =
    file:
    if file == null then
      ''${taken}import vm_pytest; vm_pytest.run(globals(), "${tests}")''
    else
      ''
        ${taken}import vm_pytest
        vm_pytest.run(globals(), "${tests}", values="${file}")
      '';
in
pkgs.testers.runNixOSTest (
  removeAttrs test [
    "tests"
    "values"
    "plugins"
  ]
  // {
    extraPythonPackages =
      python: extraPythonPackages python ++ [ (pkgs.callPackage ./vm-helpers.nix { } python) ];
    # The test framework hands a script's function only the arguments it
    # names, so this one names the nodes.
    testScript =
      if builtins.isFunction values then
        { nodes, ... }:
        script (values {
          inherit nodes;
        })
      else
        script values;
    # nix fmt holds the tests and vm_pytest to mypy --strict against the
    # driver's own types, so the driver's check of this one line would add
    # only its time.
    skipTypeCheck = true;
  }
)
