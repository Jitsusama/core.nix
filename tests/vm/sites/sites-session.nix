# sites_session.py as a module of the driver's Python, which each sites' VM
# test gives its driver with `extraPythonPackages` and takes as a plugin.
{ runCommandLocal }:
python:
python.toPythonModule (
  runCommandLocal "sites-session" { } ''
    mkdir -p $out/${python.python.sitePackages}
    cp ${./sites_session.py} $out/${python.python.sitePackages}/sites_session.py
  ''
)
