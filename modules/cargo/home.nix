# cargo's settings for every Rust build on a Linux machine. Rust links through
# the C compiler, which picks GNU ld unless told otherwise, and on a Dell XPS 14
# the link was most of a rebuild: mold took typst's one-line rebuild from 6.3 s to
# 1.1 s, and ripgrep's from 0.88 s to 0.34 s. Projects bring their own
# toolchains, so this installs none.
#
# mold comes in as the linker, a C compiler that adds it, not as rustflags:
# rustflags are part of every crate's build, so a build cache of crates built
# without them can't serve any (one refuses to run beside them at all), while
# the linker only joins the final link. A project's own linker, for its triple
# or under a cfg written as this one is, still wins, as its config is nearer.
# A -fuse-ld in a project's rustflags doesn't: mold comes last, which it must
# to beat the one rustc passes.
{
  config,
  lib,
  pkgs,
  # The machine's own settings, which Home Manager gives when it's part of
  # one, for the filesystem cargo's builds have there.
  osConfig ? { },
  ...
}:
let
  builds = osConfig.jitsusama.cargo.builds.folder or null;
  # The C compiler on PATH, as cargo would call it, with mold last so it wins
  # over any -fuse-ld before it, and told where ld.mold is so no shell has to
  # put it on PATH.
  linker = pkgs.writeShellScript "cc-with-mold" ''
    exec cc "$@" -B${pkgs.mold}/bin -fuse-ld=mold
  '';
in
{
  programs.cargo = {
    enable = true;
    package = null;
    settings = {
      # Every Linux target, as the rustflags this replaces were. cargo uses a
      # project's linker for its triple before this; a project's own cfg that
      # also matches, written otherwise, makes cargo stop and name both.
      target."cfg(target_os = \"linux\")".linker = "${linker}";
      # On a machine whose cargo has a filesystem of its own (nixos.nix says
      # why), each workspace's intermediates go there, in a folder named for
      # where the workspace is. What a build makes to keep, its programs and
      # libraries, stays in the workspace's target.
      build = lib.mkIf (builds != null) {
        build-dir = "${builds}/${config.home.username}/{workspace-path-hash}";
      };
    };
  };
}
