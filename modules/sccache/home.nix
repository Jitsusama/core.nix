# sccache under every cargo build an account starts, so a new worktree of a
# Rust workspace reuses the dependencies another worktree already compiled
# rather than compiling them again. Each worktree builds in a folder of its
# own, and cargo's own caching stops at that folder's edge; sccache keys a
# compile by its inputs instead, so it hits across them. Crates the
# workspace itself holds are built incrementally and pass straight through.
#
# It is said in the environment rather than as cargo's `build.rustc-wrapper`,
# since other tools write ~/.cargo/config.toml. A CARGO_* variable set apart
# between worktrees would miss every Rust compile, as sccache keys on them,
# so nothing here sets one. The cache is sccache's default folder, the one
# the disk guard already trims, capped so it stays a cache rather than a
# second build folder.
{ lib, pkgs, ... }:
let
  variables = {
    RUSTC_WRAPPER = lib.getExe pkgs.sccache;
    SCCACHE_CACHE_SIZE = "40G";
  };
in
{
  home.packages = [ pkgs.sccache ];
  home.sessionVariables = variables;
  systemd.user.sessionVariables = variables;
}
