# inkscape-drawings, which says what Inkscape's Recent in the launcher
# shows of each drawing Inkscape opened lately; drawings.sh says how.
{
  callPackage,
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
    # What reads the desktop's list of files opened lately.
    (callPackage ../open-with/recent-files.nix { })
  ];
  text = builtins.readFile ./drawings.sh;
}
