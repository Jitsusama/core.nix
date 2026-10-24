# What VM tests' scripts share, as modules of the driver's Python: vm_helpers.py, the helpers
# a script calls, and vm_pytest.py, which runs a test written for pytest inside the driver. A
# test gives them to its driver with `extraPythonPackages = python: [ (vmHelpers python) ];`,
# and pytest comes with them. Their tests are vm-helpers-test.py and vm-pytest-test.py.
{ stdenvNoCC }:
python:
python.toPythonModule (
  # A derivation of stdenv's rather than runCommand's, since only its fixup writes the
  # propagated-build-inputs file through which the driver's wrapper finds pytest.
  stdenvNoCC.mkDerivation {
    name = "vm-helpers";
    dontUnpack = true;
    propagatedBuildInputs = [ python.pytest ];
    installPhase = ''
      mkdir -p $out/${python.python.sitePackages}
      cp ${./vm_helpers.py} $out/${python.python.sitePackages}/vm_helpers.py
      cp ${./vm_pytest.py} $out/${python.python.sitePackages}/vm_pytest.py
    '';
  }
)
