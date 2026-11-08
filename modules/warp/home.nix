# warp says how the VPN WARP's link stands: its state and why, the
# tunnel's latency, loss, protocol, TLS, colo, endpoints, last handshake
# and totals, the settings that shape it and who set each, and what its
# DNS proxy has answered, every figure in its own unit, read from
# warp-cli, WARP's own client, each time it's run (src/main.rs says why
# it keeps nothing). `warp --json` gives the same as one record, which the
# shell's WARP reads. Where WARP isn't installed it says so and exits 3.
{ pkgs, ... }:
{
  home.packages = [ (pkgs.callPackage ./package.nix { }) ];
}
