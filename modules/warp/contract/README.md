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
| `daemon-down.json`     | any, service stopped  | warp-cli unable to reach WARP's service        |
| `refused.json`         | `registration devices` | an error, as warp-cli says any, exiting 1     |

`daemon-down.json` was seen with the service's socket hidden from
warp-cli in a namespace of its own; the link was never taken down to see
it. A status reason is a word (`NetworkHealthy`) or, as `NetworkDegraded`
is, a word holding figures under keys naming their units. What `tunnel
stats` says with no tunnel hasn't been seen, so a refusal there leaves the
tunnel unknown rather than guessing its shape.

## Where Each Figure Shows

The place is Network's WARP in the launcher, its row saying how the link
stands and its parts a step in; the page is what `warp` prints, and every
figure it shows is in `warp --json` too.

| Figure                                   | Shows                                          |
| ---------------------------------------- | ---------------------------------------------- |
| `status`                                 | place: the row and State; page: its head       |
| `reason`                                 | place: Health, or State's note; page: its head |
| a reason's figures                       | place: Health's note; page: its head           |
| `estimated_latency_ms`                   | place: the row and Latency; page               |
| `estimated_loss`                         | place: Loss, as a percentage; page             |
| `protocol`                               | place: Protocol, how it's carried its note; page |
| `edge.colo`, `edge.metal`                | place: Colo, the metal its note; page          |
| `bytes_sent`, `bytes_received`           | place: Sent and Received; page                 |
| `v4_endpoint`, `v6_endpoint`             | page; `::` is none                             |
| `secs_since_last_handshake`              | page                                           |
| `warp_is_on`                             | page, when it's off                            |
| `tls`: version, curve, cipher, post-quantum, SNI | page                                   |
| `operation_mode`, `always_on`, `switch_locked`, `auto_connect_after_min`, `organization`, `warp_tunnel_protocol`, `post_quantum_config` | page, each with who set it |
| `split_tunnel_mode`, and how many hosts and networks it lists | page          |
| every count in `dns stats`, and its average | page                                        |

## Left Out, and Why

- The drawer's, still to come: the handshake, the endpoints and TLS in the
  place, sparks of latency and loss, and the split tunnel's hosts and
  networks themselves, with `doh_hostname`, `doh_resolver_ips`,
  `fallback_domains`, `lan_subnet_size`, the captive portal's settings,
  `emergency_disconnect`, `enable_pmtud`, and `support_url` once one is
  set. Each is a setting or a detail worth a look, not how the link stands.
- For `:connect` and `:disconnect`, to come with them: `allowed_to_leave`,
  `allow_mode_switch` and `override show`, which say whether Joel may.
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
