# shots-changed, for the dev shell: what changed on screen between two runs of
# a VM test, from their pictures. shots-changed.sh says how it's used.
{
  findutils,
  imagemagick,
  writeShellApplication,
}:
writeShellApplication {
  name = "shots-changed";
  runtimeInputs = [
    findutils
    imagemagick
  ];
  text = builtins.replaceStrings [ "@moving@" ] [ "${./shots-that-move.txt}" ] (
    builtins.readFile ./shots-changed.sh
  );
}
