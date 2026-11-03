# sccache under every cargo build an account starts, so a new worktree of a
# Rust workspace reuses the dependencies another worktree already compiled
# rather than compiling them again. Each worktree builds in a folder of its
# own, and cargo's own caching stops at that folder's edge; sccache keys a
# compile by its inputs instead, so it hits across them. Crates the
# workspace itself holds are built incrementally and pass straight through.
# On 2026-11-03 a cold `cargo test --no-run` of a large workspace took 5,902
# CPU seconds plain; in a second copy at another path, with the cache warm
# from the first, it took 4,779, and again at the same path, as after the
# disk guard collects a build folder, 3,145. Filling the cache cost no CPU.
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
