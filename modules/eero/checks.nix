# The checks of eero, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# eero seals its session with systemd's credential service, and builds only
# on Linux.
onLinux {
  # eero's tests run as it builds: the contract's answers and a short
  # history read into exactly the model home-network.json holds, a read not
  # yet made reads as unknown, eero failing, refusing and a session aged
  # past refreshing are told apart, each part of the home network is read
  # at its own pace and faster while someone listens, an outage keeps the
  # last figures and says since when and why, a listener on the socket is
  # written the model on joining and on every change, the history keeps
  # and draws what moves in a database of its own in memory or a temporary
  # folder, the look at the internet is timed against listeners on this
  # machine and its outages drawn from sequences of looks, the status line says what eero said and nothing it didn't,
  # each change worth a notice between two models is told once in its words, from the internet
  # out and back to an event no other notice says, and nothing before eero's first answer,
  # what each notice said is kept and the last 30 days of it drawn into the model, at most
  # 50 and one a notice, a history from before the words were kept keeping its rows,
  # and the conferencing setting is sent as eero takes it and counted only
  # once read back. Every view's page is held to a golden one drawn from
  # home-network.json (src/views/golden/), its figures lined up by their
  # point with their units beside them, a missing one as a dash; a cloud
  # that isn't answering, a session signed out and a reading grown old are
  # said in the head with what to do; a name two devices share is refused
  # with each one's address; colour comes only on a terminal and changes no
  # character; each view's JSON is the record's own slice of it; and a
  # watch draws its view again from each model the socket writes.
  eero-reads-what-eero-says = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in eero, its tests included.
  eero-passes-clippy = passesClippy "eero-passes-clippy" (pkgs.callPackage ./package.nix { });
  # eero completes in zsh, typed in a real line editor with the real eero
  # over home-network.json and the completion home.nix installs, down its
  # tree as help lists it: each collection once and no old spelling, each
  # node by its location, each device by its name or, where another shares
  # it, its address, each profile, what a node or profile holds after its
  # name, an act's own words, the old spellings as they were, and --json
  # after a view (tests/eero-completion.zsh).
  eero-completes-in-zsh =
    pkgs.runCommandLocal "eero-completes-in-zsh"
      {
        nativeBuildInputs = [
          (pkgs.callPackage ./package.nix { })
          pkgs.zsh
        ];
      }
      ''
        export HOME=$PWD XDG_RUNTIME_DIR=$PWD/run
        mkdir -m 700 run
        mkdir run/eero
        cp ${./home-network.json} run/eero/home-network.json
        zsh -f ${./tests/eero-completion.zsh} \
          ${pkgs.callPackage ./completion.nix { }}/share/zsh/site-functions/_eero
        touch $out
      '';
}
