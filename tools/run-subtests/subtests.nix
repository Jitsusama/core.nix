# Some of a VM test's subtests, or the tests pytest picks of it, as a build
# of their own: the check, its build running run-subtests on the check's own
# driver instead of the driver on the whole script. `run-subtests --there`
# has nix-offload build this where the VM can run, so neither the machines
# nor the VM are made on the machine asking. The pictures are the output, and
# with no words the subtests' names are, in `subtests`.
{ pkgs, checks }:
{
  check,
  words ? [ ],
}:
let
  run-subtests = pkgs.callPackage ./run-subtests.nix { };
  test = checks.${check};
in
test.overrideTestDerivation (_: {
  buildCommand = ''
    mkdir -p $out
    export LOGFILE=/dev/null
    ${pkgs.lib.getExe run-subtests} --driver ${test.driver} --out $out \
      ${pkgs.lib.escapeShellArgs ([ check ] ++ words)}
  '';
})
