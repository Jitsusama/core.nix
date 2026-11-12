# The checks of chrome, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
let
  recent-sites = pkgs.callPackage ./recent-sites/package.nix { };
  session = pkgs.callPackage ./session/package.nix { };
  pack-extension = pkgs.callPackage ./pack-extension/package.nix { };
  open-tabs = pkgs.callPackage ./open-tabs/host/package.nix { };

  # Vimium as Chrome's web store has it, whose parser reads the keys.
  vimium = pkgs.fetchFromGitHub {
    owner = "philc";
    repo = "vimium";
    rev = "v2.4.2";
    hash = "sha256-i4JT2moQSVGzygC4BDAqkjioCAJiFCo5Bc5pmIAfovE=";
  };
in
onLinux {
  # clippy finds nothing in the host that tells the launcher Chrome's open
  # tabs, its tests included.
  chrome-open-tabs-passes-clippy = passesClippy "chrome-open-tabs-passes-clippy" open-tabs;

  # clippy finds nothing in what packs the launcher's Chrome extension, its
  # tests included.
  chrome-pack-extension-passes-clippy = passesClippy "chrome-pack-extension-passes-clippy" pack-extension;

  # clippy finds nothing in what reads Chrome's sites for the launcher, its
  # tests included.
  chrome-recent-sites-passes-clippy = passesClippy "chrome-recent-sites-passes-clippy" recent-sites;

  # Vimium takes the settings an account with Chrome writes it, as it
  # restores a backup: its own parser reads their keys after its defaults,
  # refusing none, and each of the key language's does the language's act,
  # with Vimium's own H, L, x, f and F beside them
  # (tests/chrome-vimium-takes-the-keys.mjs).
  chrome-vimium-takes-the-keys =
    let
      home = account (bareNixos [
        self.nixosModules.graphical
        { home-manager.users.someone.imports = [ self.homeModules.chrome ]; }
      ]);
      settings = home.xdg.configFile."jitsusama/vimium.json" or null;
    in
    pkgs.runCommandLocal "chrome-vimium-takes-the-keys" { nativeBuildInputs = [ pkgs.nodejs ]; } (
      if settings == null then
        ''
          echo "the account writes Vimium no settings"
          exit 1
        ''
      else
        ''
          node ${./tests/chrome-vimium-takes-the-keys.mjs} ${vimium} ${settings.source}
          touch $out
        ''
    );

  # clippy finds nothing in what reads Chrome's open pages from its session
  # file, its tests included.
  chrome-session-passes-clippy = passesClippy "chrome-session-passes-clippy" session;

  # Its tests run as it builds: every tab Chrome's session file holds open,
  # with its window, whether it's in front and when it was looked at, the
  # page a pruned tab went back to, none of a closed tab or window or an
  # app's, a page with a program of its own by its title alone, a command
  # cut off waiting for the next reading, and only version 3 read.
  chrome-session-reads-what-chrome-has-open = session;

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
          printf 'https://site%s.example.org/\tKept\t%s\n' $n $((1800000000 + n))
        done)
        [ "$listed" = "$expected" ] || { echo "listed:"; echo "$listed"; exit 1; }
        sha256sum -c --quiet before || { echo "reading changed Chrome's history"; exit 1; }
        [ -z "$(ls -A runtime)" ] || { echo "a copy was left behind: $(ls -A runtime)"; exit 1; }
        touch $out
      '';
}
