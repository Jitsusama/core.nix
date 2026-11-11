# What eero's Cloud Answers

Every read the eero app makes, asked once of a real three-node eero Pro 6
network on 2026-11-07 and kept here, redacted: each name, address,
serial and id is a stand-in that stays the same across files, so a
device's node in `net.devices.json` is still a node in `net.eeros.json`,
and every secret (the Wi-Fi and guest passwords, Thread's keys, the
owner's name) is gone. The parsers are tested against these files, and
the table below says what every figure means and where it shows, or why
it doesn't. A file's name is its path under `/2.2/networks/<id>/`, dots
for slashes: `net.eeros.json` is `networks/<id>/eeros`,
`eero.2002.connections.json` is `eeros/2002/connections`, and
`usage.*` is the `data_usage` family, read for the last day at hourly
cadence (devices daily).

Many entries because each file is one read's answer, named for the read
that gave it: folders would break that naming, and the tests find each
answer by the name of the read it stands for.

## What Doesn't Answer

The app's other reads, and why they're left out:

| Read                                   | Answer | Why                                     |
| -------------------------------------- | ------ | --------------------------------------- |
| `channel_utilization` (any band)       | 403    | eero Plus; each node's own figure is in `radio_channel_stats` |
| `insights/devices`, `insights/profiles` | 403   | eero Plus                               |
| `backup_access_points`, `backupinternet` | 403  | eero Plus                               |
| `eeros/<id>/ports`, `led`, `nightlight`, `support` | 404 | not on this model; ports come from `connections` |
| `settings`, `power_saving`, `burst_reporters`, `multistaticip`, `dns_policies/network/*` | 404 | not on this network's firmware or tier |
| `outages`, `devices/secondary_wan_config` | 400 | outages come from `cellular_backup_events` |

## Where Each Figure Shows

The places are: the **place** (the Home Network place's rows), a **node**'s
drawer, a **device**'s drawer, the **internet**'s drawer, a **profile**'s
drawer, the CLI's views, named as `eero <view>`, and a **notice** the
daemon raises when the figure changes while it watches
(`src/notices.rs`). Every CLI view also answers `--json`, the daemon's
model of it.

### The Internet (`network.json`, `net.speedtest.json`, `net.updates.json`)

| Figure | Means | Shows |
| ------ | ----- | ----- |
| `health.internet.status`, `isp_up` | the internet as eero sees it | place head, internet, `status`, `internet`; a notice when it goes out and comes back |
| `health.eero_network.status` | the mesh as a whole | place head, `status` |
| `status` | the network's own state | internet, `status` |
| `wan_ip`, `ip_settings.public_ip`, `double_nat` | the public address, and whether eero sits behind another NAT | internet, `internet` |
| `gateway_ip` | the LAN gateway, the gateway node | internet, `internet` |
| `lease.mode`, `lease.dhcp.{ip,mask,router}`, `wan_type` | how the internet's address is had, its mask and the ISP's router | internet, `internet` |
| `connection.mode`, `dhcp.mode`, `gateway` | NAT or bridge, and how the LAN hands out addresses | internet, `internet` |
| `dns.mode`, `dns.parent.ips`, `dns.caching`, `ipv6.name_servers.mode` | the resolvers the internet gave, and eero's DNS caching | internet, `internet`, `dns` |
| `dns.default_test_servers` | the resolvers eero offers as custom ones | left out: an offer, not a fact of the internet |
| `geo_ip.isp`, `org`, `asn`, `city`, `region`, `countryCode` | who carries the internet, and where eero places it | place head (ISP), internet, `internet` |
| `ipv6_upstream`, `ipv6_lease` | whether the internet gives IPv6 | internet, `internet` |
| `upnp`, `nat_port_randomization`, `sqm` | UPnP, port randomization, and Optimize for Conferencing and Gaming | internet, `internet`; `sqm` is `:sqm` and `eero sqm` |
| `band_steering`, `wpa3`, `wireless_mode`, `mlo_mode` | the radios' network-wide settings | internet, `internet` |
| `net.wpa3_per_band.json` `band_<GHz>_ghz` | each band's Wi-Fi security, as eero names it | internet, `internet` |
| `net.ac_compat.json` `enabled` | whether the network keeps to what older Wi-Fi 5 (802.11ac) devices can join | internet, `internet` |
| `speed.{down,up}.value`, `speed.date`, `speed.status` | the last speed test | place head, internet, `status`, `speeds` |
| `net.speedtest.json` `[].{down_mbps,up_mbps,date}` | every speed test eero kept | internet (a spark of each), `speeds` |
| `online_eeros_count`, `offline_eeros_count`, `pending_eeros_count` | the nodes by state | place head, `status` |
| `clients.count` | devices eero counts | place head, `status` |
| `updates.{target_firmware,update_to_firmware,has_update,can_update_now,preferred_update_hour,last_update_started}` | firmware: what runs, what's next, when it updates | internet, `firmware`; a notice when an update comes to wait |
| `thread`, `net.thread.json` `{name,channel,border_agent}` | the Thread network and its border agent | internet, `internet`; its keys are dropped |
| `guest_network.{enabled,name}` | the guest network | internet, `internet`; its password is dropped |
| `ddns.enabled`, `backup_internet_enabled`, `power_saving` | the rest of the internet's switches | internet, `internet` |
| `net.cellular_backup_events.json` `outages` | the outages eero recorded | internet, `outages` |
| `net.network_scan.json` `conflicting_ssid` | whether eero's last scan saw another network with this one's name | internet, `internet` |
| `net.diagnostics.json` `status` | whether a diagnostics run has begun | left out: `not_started` here, so a run's shape is unknown |
| `net.guestnetwork.json` | the guest network again | left out: `guest_network` gives it |
| `net.cellular_backup_usage.json`, `net.blacklist.json` | backup internet's traffic, and devices blocked from the network | left out: empty here, so their shape is unknown |
| `premium_status`, `capabilities.*` | eero Plus and what the network can do | `capabilities`; the place notes a figure only Plus gives |
| `flags`, `resources`, Amazon and Ring fields | eero's own wiring | left out: not about the network |

### A Node (`net.eeros.json`, `eero.<id>.json`, `eero.<id>.connections.json`)

| Figure | Means | Shows |
| ------ | ----- | ----- |
| `location`, `model`, `model_number`, `serial` | which node it is | place, node, `nodes`, `node` |
| `gateway`, `is_primary_node`, `using_wan` | whether it's the gateway node | place (drawn first), node |
| `status`, `state`, `heartbeat_ok`, `last_heartbeat` | whether it's up and when it last said so | place, node, `nodes`; a notice when it goes offline and comes back online |
| `wired`, `wireless_upstream_node.{name,primary_mesh_radio}`, `mesh_quality_bars` | its backhaul: Ethernet, or which node it hangs from, over which radio, how well | place (the mesh's edges), node, `nodes`, `mesh` |
| `ip_address`, `ipv6_addresses`, `mac_address`, `ethernet_addresses` | its addresses | node, `node` |
| `os_version`, `update_available`, `update_status.support_expiration_date` | its firmware and its support's end | node, `node`, `firmware`; a notice when `os_version` changes |
| `joined`, `last_reboot`, `uptime.since_last_reboot_s`, `uptime.since_cloud_connection_s` | its age, uptime and time on eero's cloud | node, `node` |
| `connected_clients_count`, `_wired_`, `_wireless_` | its devices | place, node, `nodes` |
| `radio_channel_stats.<band>.{channel,channel_width,tx_power,channel_utilization,client_count}` | each radio: channel, width, power, how busy, devices | node (a row per band, utilization sparked), `node`, `radio` |
| `bands`, `bssids_with_bands`, `wifi_bssids` | each radio's BSSID | node, `node`; the BSSID matches this machine's link |
| `ethernet_status.statuses[].{port_name,hasCarrier,speed,isWanPort,bandwidth_usage,neighbor}`, `wiredInternet` | each port: linked, speed, bytes, its neighbour | node (a row per port), `node` |
| connections `ports.interfaces[].{supported_speed,negotiated_speed,port_status,is_upstream}` | each port's best and current speed | node, `node` |
| connections `wireless_devices[].metadata` | the devices on it | node (its devices), `node` |
| `led_on`, `led_brightness`, `power_info.power_source`, `power_saving.schedule` | its light and power | node, `node` |
| `usage.eero.<id>.json` series, `usage.eeros.summary.json` | bytes through it, by hour | node (a spark), `traffic` |
| `resources`, `retrograde_capable`, `auto_provisioned`, `accessories` | eero's own wiring | left out |

Its upstream is named by `wireless_upstream_node.name`, the upstream's
location: its `node_or_proxied_node_id` is in another numbering than the
nodes' urls, so the mesh is joined by name.

### A Device (`net.devices.json`, `device.<mac>.json`)

| Figure | Means | Shows |
| ------ | ----- | ----- |
| `display_name`, `nickname`, `hostname` | what it's called, and by whom | place, device, `devices` |
| `device_type`, `manufacturer`, `inferred_make`, `inferred_model` | what it is | place (its mark), device, `devices` |
| `mac`, `is_private`, `eui64` | its hardware address, and whether that's a private one; this machine is the device with one of its own | device, `device`; this machine first in the place and `status` |
| `ip`, `ips`, `ipv4`, `ipv6_addresses` | its addresses | place, device, `device` |
| `connected`, `last_active`, `first_active`, `first_seen` | on now, since when, first ever; for one not on now, when it last was | place, device, `devices`; a notice when a device first seen since eero's last answer joins; each not on now in `devices` and the internet's drawer |
| `connection_type`, `wireless`, `source.{location,model,is_gateway}` | Ethernet or Wi-Fi, and through which node | place (under its node), device |
| `ssid`, `auth`, `subnet_kind`, `vlan_name`, `is_guest` | which network it joined, how | device, `device` |
| `channel`, `connectivity.frequency`, `interface.frequency`, `connectivity.bssid` | its radio: band, channel, which BSSID; `frequency` is 0 on Ethernet, read as none | place (band), device, `devices` |
| `connectivity.signal`, `snr`, `score`, `score_bars` | how well it hears and is heard | place (signal), device (sparked), `devices` |
| `connectivity.rx_bitrate`, `rx_rate_info` and `tx_rate_info` `{rate_bps,mcs,nss,channel_width,guard_interval,phy_type}` | each direction's rate and how it's had: MCS, streams, width, guard interval, Wi-Fi generation | place (rate), device (each figure), `device` |
| `connectivity.tx_retry_pct`, `inactive_time_ms`, `packet_stats.*` | retries, drops and failures per million | device, `device` |
| `connectivity.ethernet_status.{port_name,speed}` | an Ethernet device's port and link speed | device, `device` |
| `usage.{down_mbps,up_mbps}` | its speed now | place (live), device (sparked), `devices` |
| `usage.device.<mac>.json` series, `usage.devices.json` | its bytes by hour, and the day's | device (a spark), `traffic` |
| `profile`, `paused`, `blacklisted`, `dropped` | its profile, and whether it's held off | device, `device` |
| `homekit`, `ring_lte`, `amazon_devices_detail` | other makers' wiring | left out |

A device a node lists in `connections` and a device the network lists
are joined by `url`.

### People (`net.profiles.json`, `profile.<id>.json`, `net.members.json`)

| Figure | Means | Shows |
| ------ | ----- | ----- |
| `name`, `state.value`, `paused`, `default`, `schedule` | a profile and whether it's held off now; `schedule` as how many | place (a device's profile), profile, `profiles` |
| `state.schedule`, `resources.schedules` | the schedule holding it off now, and where its schedules are read | left out: `null` here, so its shape is unknown, and no schedule to read |
| `devices[]` | the devices in it | profile, `profile` |
| `unified_content_filters`, `premium_dns.{dns_policies,ad_block_settings,advanced_content_filters,blocked_applications}` | what it blocks and lets through; a kind the unified filters leave out is `premium_dns`'s | profile, `profile`, `profiles` |
| `premium_dns.{zscaler_location_enabled,dns_provider}` | who answers its filtered lookups | left out: off and `null` here |
| `proxied_nodes` | not known | left out: empty here |
| `usage.profile.<id>.json` series | its bytes by hour | profile (a spark), `traffic` |
| `members[].user_role` | who may change the network | internet, `internet`; names are dropped |

### Addresses (`net.subnets_config.json`, `net.routing.json`)

| Figure | Means | Shows |
| ------ | ----- | ----- |
| `[].{subnet_kind,name,enabled}` | each subnet: the main or guest network, its name, whether it's on | internet, `internet` |
| `[].dedicated_subnet.{subnet_ip,subnet_mask,start_ip,end_ip}` | its address, as a prefix, and the range DHCP hands out | internet, `internet` |
| `[].{wan_access,open_network}` | whether its devices reach the internet, and whether it asks no password | internet, `internet` |
| `[].lan_access` | not known | left out: false for the main network too, so what it means isn't clear |
| `[].{subnet_type,subnet_label,nat_port_randomization}` | `subnet_kind` again, a label the app never shows, and the network's own switch | left out: said elsewhere or never shown |
| `routing` `reservations.data[].{mac,description,ip}` | an address kept for a device, and the label it was given | internet, `internet`, the device it names, `device` |
| `routing` `forwards.data`, `pinholes.data` | ports forwarded in, and IPv6 pinholes | internet, `internet`, in eero's own words, since none here gives their shape |
| `routing` `devices.data[].{mac,nickname}` | devices by nickname | left out: 29 here, repeating names the device reads give |
| `net.reservations.json`, `net.forwards.json` | `routing`'s own lists again | left out: `routing` gives them in one read |

### What Happened (`net.app_events.json`)

| Figure | Means | Shows |
| ------ | ----- | ----- |
| `events[].{timestamp_ms,category,description}` | the app's activity list: a device joining or leaving a node, with its band and channel | device (its roams), internet (a timeline), `events`; a notice for one that says what no other notice does |

Every event this network gave is a `Device` one, a device connecting to
one node or disconnecting from another as it roams, so none raises a
notice of its own: roaming and reconnecting say nothing, and a device new
to the home network is told from its `first_seen`. A `Network` or `eero`
event is told unless it names something going offline, online,
connecting, joining, updating or rebooting, or the internet, which the
notices tell from eero's own figures, so one change never raises two.

The internet's timeline, and `eero events`, set these beside what the
daemon told, the model's `told`: none of it is eero's, so this contract
holds none, and the history (`src/history/`) keeps each notice's
summary and body for 30 days. `home-network.json` holds five, told over
the days its own outage and firmware cover.
