# cargo's settings for every Rust build on a Linux machine. Rust links through
# the C compiler, which picks GNU ld unless told otherwise, and on a Dell XPS 14
# the link was most of a rebuild: mold took typst's one-line rebuild from 6.3 s to
# 1.1 s, and ripgrep's from 0.88 s to 0.34 s. Projects bring their own
# toolchains, so this installs none.
{ pkgs, ... }:
{
  programs.cargo = {
    enable = true;
    package = null;
    settings.target."cfg(target_os = \"linux\")".rustflags = [
      "-C"
      "link-arg=-fuse-ld=mold"
      # Where the compiler finds ld.mold, so no shell has to put it on PATH.
      "-C"
      "link-arg=-B${pkgs.mold}/bin"
    ];
  };
}
