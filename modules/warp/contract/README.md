# What warp-cli Says

Each file is an answer `warp-cli --json` gave, its shape as WARP's client
2026.7 gives it on Linux and its values made up: hosts under example.com,
addresses in the documentation ranges, the organisation `example`. The
tests read them as warp-cli's answers (`src/contract.rs`) and hold what
`read` draws from them to `../warp.json`.

| File                   | Asked                 | What it is                                     |
| ---------------------- | --------------------- | ---------------------------------------------- |
| `status.json`          | `status`              | the link connected, the network healthy        |
| `status.degraded.json` | `status`              | connected, a reason carrying WARP's figures    |
| `tunnel-stats.json`    | `tunnel stats`        | the tunnel's figures                           |
| `settings.json`        | `settings`            | every setting, and who set each (`sources`)    |
| `dns-stats.json`       | `dns stats`           | what the DNS proxy answered on this connection |
| `override-show.json`   | `override show`       | no administrator's override in force           |
| `daemon-down.json`     | any, service stopped  | warp-cli unable to reach WARP's service        |
| `refused.json`         | `registration devices` | an error, as warp-cli says any, exiting 1     |
| `listen-status.jsonl`  | `--listen status`     | a line each change of state, held open         |

`daemon-down.json` was seen with the service's socket hidden from
warp-cli in a namespace of its own; the link was never taken down to see
it. A status reason is a word (`NetworkHealthy`) or, as `NetworkDegraded`
is, a word holding figures under keys naming their units. What `tunnel
stats` says with no tunnel hasn't been seen, so a refusal there leaves the
tunnel unknown rather than guessing its shape.

`--listen status` was seen saying its first line, the link as it stands,
at once, one line of JSON with no spaces, the degraded line's shape
being that one; the healthy and disconnected lines are `status.json`'s
shapes written the same way, and the third the degraded link's figures
moved.

What `warp-cli connect` and `disconnect` print hasn't been seen with
`--json`, since neither is run on a machine in use to find out: without
it each prints `Success`. So `warp connect` and `warp disconnect`
(`src/switch.rs`) take a run that ends well as done and an error as
warp-cli says any, `{ code, error }`, as refused, and leave how the link
then goes to its status.

## Where Each Figure Shows

The place is Network's Work Network in the launcher, its row saying how the link
stands and its parts a step in; the page is what `warp` prints, and every
figure it shows is in `warp --json` too.

| Figure                                   | Shows                                          |
| ---------------------------------------- | ---------------------------------------------- |
| `status`                                 | place: the row and State; page: its head       |
| `reason`                                 | place: Health, or State's note; page: its head |
| a reason's figures                       | place: Health's note; page: its head           |
| `estimated_latency_ms`                   | place: the row and Round Trip, sparked with its drawer while shown; page |
| `estimated_loss`                         | place: Loss, as a percentage, sparked with its drawer while shown; page |
| `protocol`                               | place: Protocol, how it's carried its note; page |
| `edge.colo`, `edge.metal`                | place: Data Centre, the metal its note; page   |
| `bytes_sent`, `bytes_received`           | place: Traffic, in and out, its drawer the speed each way between reads; page |
| `v4_endpoint`, `v6_endpoint`             | place: Endpoint; page; `::` is none            |
| `secs_since_last_handshake`              | place: Last Handshake; page                    |
| `warp_is_on`                             | place: Tunnel, when it's off; page             |
| `tls`: version, curve, cipher, post-quantum, SNI | place: TLS, a step in; page            |
| `operation_mode`, `always_on`, `switch_locked`, `auto_connect_after_min`, `organization`, `warp_tunnel_protocol`, `post_quantum_config` | place: Settings, a step in; page; each with who set it |
| `split_tunnel_mode`, how many hosts and networks it lists, and each with what it's for | place: Settings' Split Tunnel, each a step in; page |
| `doh_hostname`, `doh_resolver_ips`, `fallback_domains`, the captive portal's settings, `enable_pmtud` | place: Settings, each list a step in; page |
| `allow_mode_switch`, `allowed_to_leave`, `support_url` once one is set, `lan_subnet_size`, `override show` | place: Settings; page |
| `emergency_disconnect`                   | place: Emergency Disconnect, right after State, while it holds; page |
| every count in `dns stats`, and its average | place: DNS, a step in; page                 |

## Left Out, and Why

- `switch_locked`, `always_on` and `auto_connect_after_min` say, besides,
  what `warp connect` and `warp disconnect` do (`../src/switch.rs`), which
  the place's switch runs as `:connect` and `:disconnect`, its note saying
  who holds a held switch and when WARP always on comes back.
- Identifiers that mean nothing read: `profile_id`, and `registration
  show`'s ids and public key. Its organisation is `settings`' own.
- About the app, not the link: `onboarding`, `allow_updates`,
  `config_type`, `register_interface_ip_with_dns`, `anycast_split_ips`
  (WARP's own addresses, routed one way always) and
  `speed_test_settings` (the servers a speed test would use).
- Not for Linux: `sccm_vpn_boundary_support` (Windows' SCCM), and
  `disable_for_wifi`, `disable_for_ethernet` and `disable_for_ssids`, which
  warp-cli says are a consumer account's.
- Said elsewhere already: `debug network`'s interface, Wi-Fi network,
  gateway and name servers are Network's Address and Wi-Fi; `dns_policy`
  repeats `doh_resolver_ips`; `settings support-url` and `settings
  mode-switch-allowed` repeat `settings`.
- The service's own workings: `stats` and `stats enhanced` count its API
  calls, IPC, happy eyeballs and the rest, about the client rather than the
  link; `debug dex` holds the organisation's own HTTP tests, and `vnet`,
  `target list` and `certs` its networks, targets and certificates.
