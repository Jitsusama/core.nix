# treefmt's mypy (mypy.sh), from the Python given, reading types from the
# folders on the types path given. treefmt.nix gives a Python carrying a
# repository's libraries and the test driver's types; the check, which asks
# only its help, gives a bare one.
{
  python,
  typesPath ? "",
  writeShellApplication,
}:
writeShellApplication {
  name = "mypy";
  text = builtins.replaceStrings [ "@python@" "@path@" ] [ "${python}" typesPath ] (
    builtins.readFile ./mypy.sh
  );
}
