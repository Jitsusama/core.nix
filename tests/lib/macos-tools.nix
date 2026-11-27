# A check that runs a script with macOS's own tools, for what a Mac runs:
# in a darwin build stdenv puts GNU's coreutils, findutils, sed and grep
# first on the PATH, so a check of a Mac's script would otherwise test GNU's
# tools in place of the ones the Mac runs it with. The script runs with
# macOS's folders alone on its PATH, under the UTF-8 locale a Mac's terminal
# runs, as `macosTools { name = ...; script = ./....sh; args = [ ... ]; }`,
# given to every folder's checks beside `onMacos`. Its shell is nixpkgs'
# bash, as the Mac's own scripts here are run, not macOS's bash 3.2.
{
  bash,
  lib,
  runCommandLocal,
  stdenv,
}:
{
  name,
  script,
  args ? [ ],
}:
assert lib.assertMsg stdenv.hostPlatform.isDarwin
  "${name} runs macOS's own tools, which only a Mac has.";
runCommandLocal name
  {
    # macOS's tools live in /usr/bin and /bin, which the build sandbox hides
    # and a Mac's allowed-impure-host-deps doesn't admit, so this builds
    # outside it. A Mac whose build sandbox is false or relaxed runs it; one
    # whose sandbox is true fails the build, rather than test GNU's tools in
    # their place. Outside it the script sees the Mac's own files and
    # network, so a check writes only to $TMPDIR and $out. Nothing of the
    # Mac's tools is in the hash, so a pass stands for the macOS that built
    # it until the check changes, or is built again with --rebuild.
    __noChroot = true;
    LANG = "en_US.UTF-8";
  }
  ''
    export PATH=/usr/bin:/bin:/usr/sbin:/sbin
    ${lib.getExe bash} ${lib.escapeShellArg "${script}"} ${lib.escapeShellArgs args}
    touch $out
  ''
