# The checks of chrome, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
let
  recent-sites = pkgs.callPackage ./recent-sites/package.nix { };
  pack-extension = pkgs.callPackage ./pack-extension/package.nix { };
in
onLinux {
  # clippy finds nothing in what packs the launcher's Chrome extension, its
  # tests included.
  chrome-pack-extension-passes-clippy = passesClippy "chrome-pack-extension-passes-clippy" pack-extension;

  # clippy finds nothing in what reads Chrome's sites for the launcher, its
  # tests included.
  chrome-recent-sites-passes-clippy = passesClippy "chrome-recent-sites-passes-clippy" recent-sites;

  # Chrome's sites are read whole from a history caught halfway through a
  # write, as Chrome leaves it while it browses: the half-written pages in
  # the file, the journal beside it. Reading rolls the write back on a copy,
  # so it lists what was last written whole, leaves Chrome's files alone and
  # leaves no copy behind (mid-write.py).
  chrome-recent-sites-reads-a-history-mid-write =
    pkgs.runCommandLocal "chrome-recent-sites-reads-a-history-mid-write"
      { nativeBuildInputs = [ pkgs.python3 ]; }
      ''
        mkdir -p profile runtime
        python3 ${./mid-write.py} profile
        sha256sum profile/History profile/History-journal > before
        listed=$(XDG_RUNTIME_DIR=$PWD/runtime ${pkgs.lib.getExe recent-sites} profile)
        expected=$(for n in 1999 1998 1997 1996 1995 1994 1993 1992 1991; do
          printf 'https://site%s.example.org/\tKept\n' $n
        done)
        [ "$listed" = "$expected" ] || { echo "listed:"; echo "$listed"; exit 1; }
        sha256sum -c --quiet before || { echo "reading changed Chrome's history"; exit 1; }
        [ -z "$(ls -A runtime)" ] || { echo "a copy was left behind: $(ls -A runtime)"; exit 1; }
        touch $out
      '';
}
