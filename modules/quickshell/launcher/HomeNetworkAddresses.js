// How the home network has and hands out addresses, as the internet's
// drawer says it under Addressing: the public address, the lease, the
// gateway and the resolvers, then each subnet and the range DHCP hands
// out, and what's let in from outside; the address kept for a device, in
// its own drawer; and each band's security, which the drawer's settings say. A list
// that's empty says none, since none is the figure. Plain reading of the
// record, held in Qt's own engine (tests/home-network.qml).
.pragma library
.import "HomeNetwork.js" as Home
.import "HomeNetworkHistory.js" as History

const DOT = " \u00b7 ";
const known = History.known;
const joined = History.joined;
const fact = History.fact;
const kept = History.kept;
const onOff = History.onOff;
// A mask is four octets of eight bits each.
const OCTETS = 4;
const OCTET = 8;
const BINARY = 2;
const OCTET_WRITTEN = /^\d{1,3}$/;
const LEADING_ONES = /^1*/;
const ZEROS = /^0*$/;

// The Addressing heading's facts, how the internet's address is had and
// then how the home network hands its own out.
function addressing(model) {
    const net = model?.internet ?? {};
    const lease = net.lease ?? {};
    const dns = net.dns ?? {};
    const addresses = net.addresses ?? {};
    return kept([
        { heading: "addressing" },
        fact("public", joined([net.public_address, net.double_nat ? "behind another NAT" : ""])),
        fact("lease", joined([net.wan_type, withPrefix(lease.address, lease.mask),
            lease.router ? `router ${lease.router}` : ""])),
        fact("gateway", net.gateway_address),
        fact("mode", joined([String(net.mode ?? "").toUpperCase(),
            net.dhcp ? `addresses ${net.dhcp}` : ""])),
        fact("dns", joined([dns.mode, (dns.servers ?? []).join(DOT),
            known(dns.caching) ? `caching ${onOff(dns.caching)}` : "",
            dns.ipv6_mode ? `IPv6 ${dns.ipv6_mode}` : ""])),
        fact("ipv6", onOff(net.ipv6)),
    ].concat((addresses.subnets ?? []).map(subnet), [
        fact("forwards", eerosWords(addresses.forwards)),
        fact("pinholes", eerosWords(addresses.pinholes)),
    ]));
}

// An address with its mask as a prefix, "10.0.0.0/22", as every address and
// mask here is said, the mask whole only where it's no prefix.
function withPrefix(address, mask) {
    const bits = prefix(mask);
    return known(address) && bits !== null ? `${address}/${bits}`
        : joined([address, known(mask) ? `mask ${mask}` : ""]);
}

// A subnet, by its kind: how it stands first, a part that isn't so giving
// way, then its address and prefix and the range DHCP hands out:
// "off \u00b7 10.0.0.0/21 \u00b7 DHCP 10.0.0.20 to 10.0.0.254" (VKCF3Z, 2026-11-08).
function subnet(one) {
    const value = joined([
        one.on === false ? "off" : "",
        one.internet === false ? "no internet" : "",
        one.open === true ? "open" : "",
        withPrefix(one.address, one.mask),
        known(one.first) && known(one.last) ? `DHCP ${one.first} to ${one.last}` : "",
    ]);
    if (one.kind === "main" || one.kind === "guest") {
        return fact(`${one.kind}-subnet`, value);
    }
    return known(value) ? { label: one.name ?? "subnet", value } : null;
}

// The prefix length a mask such as 255.255.252.0 is, null for one whose
// ones don't run unbroken from the left.
function prefix(mask) {
    const parts = String(mask ?? "").split(".");
    if (parts.length !== OCTETS || parts.some((part) => !OCTET_WRITTEN.test(part))) {
        return null;
    }
    const bits = parts.map((part) => Number(part).toString(BINARY).padStart(OCTET, "0")).join("");
    const ones = LEADING_ONES.exec(bits)[0].length;
    return bits.length === OCTETS * OCTET && ZEROS.test(bits.slice(ones)) ? ones : null;
}

// What's let in from outside, in eero's own words, since none has given
// its shape yet; none where eero lists nothing.
function eerosWords(list) {
    if (!Array.isArray(list)) {
        return "";
    }
    return list.length === 0 ? "none" : list.map((one) => JSON.stringify(one)).join(DOT);
}

// A device's address, said reserved where it's the one kept for it, and
// the kept one as a fact of its own only where they differ, so no figure
// is said twice (VKCF3Z, 2026-11-08).
function deviceAddress(model, one) {
    const same = (each) => String(each.mac).toLowerCase() === String(one.mac).toLowerCase();
    const its = (model?.internet?.addresses?.reservations ?? []).find(same)?.address ?? "";
    return [
        fact("address", joined([one.address, its !== "" && its === one.address
            ? "reserved" : ""])),
        fact("reserved", its === one.address ? "" : its),
    ];
}

// The bands' security as its acronym: one word where every band agrees,
// "WPA2", else band by band, lowest first, "2.4 GHz WPA2 \u00b7 5 GHz WPA3".
function security(bands) {
    const said = (bands ?? []).filter((one) => known(one.security));
    const words = said.map((one) => String(one.security).toUpperCase());
    if (words.length > 0 && words.every((word) => word === words[0])) {
        return words[0];
    }
    return joined(said.map((one, at) => `${one.band} ${words[at]}`));
}

// Whether eero's last scan saw another network with this one's name.
function nameClash(clash) {
    if (!known(clash)) {
        return "";
    }
    return clash ? "another network has its name" : "none seen";
}
