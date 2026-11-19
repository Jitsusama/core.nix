// The cases of addresses for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Addresses.js" as Addresses

// What's typed, and fields its reading must have.
const addresses = ({
    "10.0.0.0/22": {
        version: 4, usable: 1022, first: "10.0.0.1", last: "10.0.3.254",
        broadcast: "10.0.3.255", mask: "255.255.252.0", network: "10.0.0.0/22"
    },
    "192.168.4.65/22": { network: "192.168.4.0/22", aligned: false },
    "8.8.8.8": { version: 4, prefix: null, text: "8.8.8.8" },
    "10.1.2.3/31": { usable: 2, first: "10.1.2.2", last: "10.1.2.3" },
    "0.0.0.0/0": { size: 4294967296, mask: "0.0.0.0" },
    "2001:db8::/48": {
        version: 6, bits: 80, first: "2001:db8::", last: "2001:db8:0:ffff:ffff:ffff:ffff:ffff",
        full: "2001:0db8:0000:0000:0000:0000:0000:0000"
    },
    "2001:0DB8:0000:0000:0001:0000:0000:0001": { text: "2001:db8::1:0:0:1" },
    "fe80::15b3:5aba:b3d4:6834%wlp0s20f3": { text: "fe80::15b3:5aba:b3d4:6834" },
    "::ffff:192.0.2.1": { text: "::ffff:c000:201" },
    "::": { text: "::" },
    "1:0:0:2:0:0:0:3": { text: "1:0:0:2::3" }
});

// What's typed, and the kind of range it's in.
const kinds = ({
    "8.8.8.8": "public",
    "10.20.30.40": "private",
    "172.31.255.255": "private",
    "172.32.0.0": "public",
    "100.104.118.2": "shared, behind carrier NAT",
    "127.0.0.1": "loopback",
    "192.0.2.7": "documentation only",
    "224.0.0.251": "multicast",
    "2606:4700::1111": "public",
    "2001:db8::1": "documentation only",
    "fd01:5ca1:ab1e:8cda::1": "unique local",
    "fe80::1": "link-local",
    "::1": "loopback",
    "ff02::1": "multicast"
});

const notAddresses = ["1.2.3", "256.1.1.1", "1.2.3.4/33", "1:2:3", "a::b::c",
    "12345::", "kitty", "10.0.0.0/", "::/129"];

// What's typed, and fields a hardware address's reading must have.
const macs = ({
    "3c:22:fb:12:34:56": {
        oui: "3C22FB", dotted: "3c22.fb12.3456", local: false, multicast: false
    },
    "3C-22-FB-12-34-56": { colon: "3c:22:fb:12:34:56" },
    "3c22.fb12.3456": { colon: "3c:22:fb:12:34:56" },
    "da:a1:19:00:00:01": { local: true },
    "01:00:5e:00:00:fb": { multicast: true }
});

const notMacs = ["3c:22:fb:12:34", "3c:22-fb:12:34:56", "gg:22:fb:12:34:56",
    "3c22fb123456"];

function run(failures) {
    let asked = 0;
    const quoted = Common.quoted;
    for (const typed of Object.keys(addresses)) {
        asked++;
        const got = Addresses.read(typed);
        for (const field of Object.keys(addresses[typed])) {
            if (!got || got[field] !== addresses[typed][field]) {
                failures.push(`"${typed}" read ${field} as ${quoted(got?.[field])}, `
                    + `not ${quoted(addresses[typed][field])}`);
            }
        }
    }
    for (const typed of Object.keys(kinds)) {
        asked++;
        const got = Addresses.read(typed);
        if (!got || got.kind.kind !== kinds[typed]) {
            failures.push(`"${typed}" is ${quoted(got?.kind.kind)}, `
                + `not "${kinds[typed]}"`);
        }
    }
    for (const typed of notAddresses) {
        asked++;
        if (Addresses.read(typed)) {
            failures.push(`"${typed}" read as an address, but it isn't one`);
        }
    }
    for (const typed of Object.keys(macs)) {
        asked++;
        const got = Addresses.mac(typed);
        for (const field of Object.keys(macs[typed])) {
            if (!got || got[field] !== macs[typed][field]) {
                failures.push(`"${typed}" read ${field} as ${quoted(got?.[field])}, `
                    + `not ${quoted(macs[typed][field])}`);
            }
        }
    }
    for (const typed of notMacs) {
        asked++;
        if (Addresses.mac(typed)) {
            failures.push(`"${typed}" read as a hardware address, but it isn't one`);
        }
    }
    return asked;
}
