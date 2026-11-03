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
# The wrapper is said in the environment rather than as cargo's
# `build.rustc-wrapper`, since other tools write ~/.cargo/config.toml, and a
# project's dev shell that sets RUSTC_WRAPPER itself wins inside that shell.
# A CARGO_* variable set apart between worktrees would miss every Rust
# compile, as sccache keys on them, so nothing here sets one.
#
# Left to itself, the first sccache to run starts the server in its own
# cgroup, and every later build's compiles would run there, at whatever
# weight that first build had. So the server is a service of its own among
# the jobs, on one socket every build names, and each build compiles in its
# own process (client-side mode), keeping its own place on the machine; the
# server only reads and writes the cache. The cache is sccache's default
# folder, the one the disk guard trims as `sccache`, capped at 10 GiB: that
# workspace's took about 1 GiB.
{ lib, pkgs, ... }:
let
  sccache = lib.getExe pkgs.sccache;
  socket = "sccache.sock";
  # What a build needs to find the server and compile in its own process.
  # Shells expand $XDG_RUNTIME_DIR as they start, and the user manager its
  # ${XDG_RUNTIME_DIR}, since neither is known before login.
  client = runtime: {
    RUSTC_WRAPPER = sccache;
    SCCACHE_SERVER_UDS = "${runtime}/${socket}";
    SCCACHE_CLIENT_SIDE = "1";
  };
in
{
  home.packages = [ pkgs.sccache ];
  home.sessionVariables = client "$XDG_RUNTIME_DIR";
  systemd.user.sessionVariables = client "\${XDG_RUNTIME_DIR}";

  systemd.user.services.sccache = {
    Unit.Description = "The compile cache every cargo build reads, among the jobs";
    Install.WantedBy = [ "default.target" ];
    Service = {
      Slice = "jobs.slice";
      Environment = [
        "SCCACHE_SERVER_UDS=%t/${socket}"
        "SCCACHE_CACHE_SIZE=10G"
        # It stays up, so no build ever finds it gone and starts its own.
        "SCCACHE_IDLE_TIMEOUT=0"
        # The server itself, in the foreground, as sccache runs it when it
        # starts one: `--start-server` would fork it off and return.
        "SCCACHE_START_SERVER=1"
        "SCCACHE_NO_DAEMON=1"
      ];
      ExecStart = sccache;
      Restart = "always";
      RestartSec = 1;
    };
  };
}
