# What VM tests' scripts share (vm_helpers.py), as a module of the driver's Python: a test
# gives it to its driver with `extraPythonPackages = python: [ (vmHelpers python) ];`, and
# its script imports vm_helpers. Its tests are vm-helpers-test.py.
{ runCommandLocal }:
python:
python.toPythonModule (
  runCommandLocal "vm-helpers" { } ''
    mkdir -p $out/${python.python.sitePackages}
    cp ${./vm_helpers.py} $out/${python.python.sitePackages}/vm_helpers.py
  ''
)
