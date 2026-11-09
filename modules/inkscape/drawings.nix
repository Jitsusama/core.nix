# inkscape-drawings, which says what Inkscape's Recent in the launcher
# shows of each drawing Inkscape opened lately; drawings.sh says how.
{
  writeShellApplication,
  coreutils,
  gnugrep,
  gzip,
}:
writeShellApplication {
  name = "inkscape-drawings";
  runtimeInputs = [
    coreutils
    gnugrep
    gzip
  ];
  text = builtins.readFile ./drawings.sh;
}
