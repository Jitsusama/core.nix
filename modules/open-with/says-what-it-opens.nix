# A program that runs no scripts of its own, wrapped so its command says
# what it was given to the desktop's list of files opened lately
# (recent-files, where the desktop has it) before the program starts. It
# hands on everything it was given, and recent-files keeps only the files,
# so the program's options and a web address pass over. A file opened from
# inside the program goes unsaid; a program that can, says those itself, as
# Neovim and mpv do.
{
  lib,
  makeWrapper,
  symlinkJoin,
}:
{
  # The package to wrap, the command in it to wrap, and the name the list
  # gives the program, which is its command unless said otherwise.
  package,
  command ? package.meta.mainProgram or (lib.getName package),
  program ? command,
}:
# It keeps the package's name, so what looks for the program by its name
# still finds it.
symlinkJoin {
  inherit (package) name;
  paths = [ package ];
  nativeBuildInputs = [ makeWrapper ];
  passthru = (package.passthru or { }) // {
    unwrapped = package;
  };
  meta = package.meta or { };
  postBuild = ''
    wrapProgram $out/bin/${command} \
      --run 'if command -v recent-files >/dev/null; then recent-files add ${program} "$@" || true; fi'
  '';
}
