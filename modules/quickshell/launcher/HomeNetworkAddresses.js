// How the home network has and hands out addresses, as the internet's
// drawer says it under Addressing: the public address, the lease, the
// gateway and the resolvers, then each subnet and the range DHCP hands
// out, each address kept for a device, and what's let in from outside;
// and each band's security, which the drawer's settings say. A list
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
        fact("lease", joined([net.wan_type, lease.address && lease.mask
            ? `${lease.address} / ${lease.mask}` : lease.address,
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
    ])).concat(reserved(model));
}

// A subnet, by its kind, as its address and prefix, the range DHCP hands
// out and what sets it apart, a part that isn't so giving way:
// "10.0.0.0/22 · DHCP 10.0.0.20 to 10.0.0.254".
function subnet(one) {
    const bits = prefix(one.mask);
    const value = joined([
        known(one.address) && bits !== null ? `${one.address}/${bits}`
            : joined([one.address, known(one.mask) ? `mask ${one.mask}` : ""]),
        known(one.first) && known(one.last) ? `DHCP ${one.first} to ${one.last}` : "",
        one.on === false ? "off" : "",
        one.internet === false ? "no internet" : "",
        one.open === true ? "open" : "",
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

// The addresses kept for a device, each by the device's own name where
// eero lists it, on now or not, else the label the reservation was given,
// and that label too when it says more; none where eero keeps none.
function reserved(model) {
    const list = model?.internet?.addresses?.reservations;
    if (!Array.isArray(list)) {
        return [];
    }
    if (list.length === 0) {
        return [fact("reserved", "none")];
    }
    return [{ heading: "reserved" }].concat(list.map((one) => {
        const device = listed(model, one.mac);
        const name = device ? Home.nameOf(device) : one.description || one.mac;
        return {
            label: name,
            value: joined([one.address,
                device && one.description && one.description !== name ? one.description : ""]),
        };
    }));
}

// The device eero lists with a hardware address, on now or not.
function listed(model, mac) {
    const same = (each) => String(each.mac).toLowerCase() === String(mac).toLowerCase();
    return (model?.devices ?? []).find(same) ?? (model?.devices_off ?? []).find(same) ?? null;
}

// The address kept for a device, or nothing where none is.
function reservedFor(model, one) {
    const same = (each) => String(each.mac).toLowerCase() === String(one.mac).toLowerCase();
    return (model?.internet?.addresses?.reservations ?? []).find(same)?.address ?? "";
}

// Each band's security, lowest band first, as its acronym:
// "2.4 GHz WPA2 · 5 GHz WPA2".
function security(bands) {
    return joined((bands ?? []).map((one) => known(one.security)
        ? `${one.band} ${String(one.security).toUpperCase()}` : ""));
}

// Whether eero's last scan saw another network with this one's name.
function nameClash(clash) {
    if (!known(clash)) {
        return "";
    }
    return clash ? "another network has its name" : "none seen";
}
