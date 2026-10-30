# The command a file source's `opened` is (recents.nix) for a program
# whose processes each keep the file they have open in a folder, as a
# file named by the process's id holding its id, a tab and the file's
# whole path: it prints every one's line, passing over what one is still
# writing aside, under a name starting with a dot. The launcher runs it
# without a shell, so it's the folder's whole path.
{ lib, pkgs }:
folder: [
  (lib.getExe' pkgs.findutils "find")
  folder
  "-name"
  "[0-9]*"
  "-type"
  "f"
  "-exec"
  (lib.getExe' pkgs.coreutils "cat")
  "--"
  "{}"
  "+"
]
