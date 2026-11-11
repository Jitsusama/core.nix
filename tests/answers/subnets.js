// The cases of subnets for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Subnets.js" as Subnets

// What's typed, the answer's headline, what Return copies, and its
// parts, each its id (or a split's subnet) and what it says.
var subnets = [
    ["10.0.0.0/22", "1,022 hosts", "10.0.0.0/22", ["range 10.0.0.0 \u2013 10.0.3.255",
        "mask 255.255.252.0", "wildcard 0.0.3.255", "network 10.0.0.0/22",
        "broadcast 10.0.3.255", "first 10.0.0.1", "last 10.0.3.254",
        "size 1,024 addresses"]],
    ["10.0.0.0 255.255.252.0", "1,022 hosts", "10.0.0.0/22", [
        "range 10.0.0.0 \u2013 10.0.3.255", "mask 255.255.252.0", "wildcard 0.0.3.255",
        "network 10.0.0.0/22", "broadcast 10.0.3.255", "first 10.0.0.1", "last 10.0.3.254",
        "size 1,024 addresses"]],
    ["10.0.0.5/22", "in 10.0.0.0/22", "10.0.0.0/22", ["range 10.0.0.0 \u2013 10.0.3.255",
        "mask 255.255.252.0", "wildcard 0.0.3.255", "network 10.0.0.0/22",
        "broadcast 10.0.3.255", "first 10.0.0.1", "last 10.0.3.254", "hosts 1,022",
        "size 1,024 addresses"]],
    ["ip 10.1.2.3/31", "in 10.1.2.2/31", "10.1.2.2/31", ["range 10.1.2.2 \u2013 10.1.2.3",
        "mask 255.255.255.254", "wildcard 0.0.0.1", "network 10.1.2.2/31",
        "first 10.1.2.2", "last 10.1.2.3", "hosts 2", "size 2 addresses"]],
    ["is 10.1.2.3 in 10.0.0.0/16", "no", "no", ["network 10.0.0.0/16",
        "range 10.0.0.0 \u2013 10.0.255.255"]],
    ["is 10.0.2.3 in 10.0.0.0 255.255.0.0?", "yes", "yes", ["network 10.0.0.0/16",
        "range 10.0.0.0 \u2013 10.0.255.255"]],
    ["is 10.0.4.0/24 in 10.0.0.0/22", "no", "no", ["network 10.0.0.0/22",
        "range 10.0.0.0 \u2013 10.0.3.255"]],
    ["is 2001:db8::5 in 2001:db8::/32", "yes", "yes", ["network 2001:db8::/32",
        "range 2001:db8:: \u2013 2001:db8:ffff:ffff:ffff:ffff:ffff:ffff"]],
    ["/22", "255.255.252.0", "255.255.252.0", ["wildcard 0.0.3.255", "hosts 1,022",
        "size 1,024 addresses"]],
    ["/48", "65,536 /64s", "65,536 /64s", ["size 2^80 addresses"]],
    ["10.0.0.0/22 in /24s", "4 /24s", "10.0.0.0/24\n10.0.1.0/24\n10.0.2.0/24\n10.0.3.0/24", [
        "10.0.0.0/24 10.0.0.0 \u2013 10.0.0.255", "10.0.1.0/24 10.0.1.0 \u2013 10.0.1.255",
        "10.0.2.0/24 10.0.2.0 \u2013 10.0.2.255", "10.0.3.0/24 10.0.3.0 \u2013 10.0.3.255"]],
    ["2001:db8::/63 into /64s", "2 /64s", "2001:db8::/64\n2001:db8:0:1::/64", [
        "2001:db8::/64 2001:db8:: \u2013 2001:db8::ffff:ffff:ffff:ffff",
        "2001:db8:0:1::/64 2001:db8:0:1:: \u2013 2001:db8:0:1:ffff:ffff:ffff:ffff"]],
    ["2001:db8::/48", "65,536 /64s", "2001:db8::/48", [
        "range 2001:db8:: \u2013 2001:db8:0:ffff:ffff:ffff:ffff:ffff",
        "size 2^80 addresses", "full 2001:0db8:0000:0000:0000:0000:0000:0000"]],
    ["2001:db8::1/48", "in 2001:db8::/48", "2001:db8::/48", [
        "range 2001:db8:: \u2013 2001:db8:0:ffff:ffff:ffff:ffff:ffff",
        "network 2001:db8::/48", "subnets 65,536 /64s", "size 2^80 addresses",
        "full 2001:0db8:0000:0000:0000:0000:0000:0001"]]
];

var notSubnets = ["10.0.0.0 255.255.253.0", "/129", "is 10.0.0.1 in",
    "10.0.0.0/24 in /22s", "10.0.0.0/22 in /33s", "is kitty in 10.0.0.0/8", "/", "22"];

function run(failures, fromRoot) {
    let asked = 0;
    const Maths = fromRoot.maths;
    const quoted = Common.quoted;
    const parted = Common.parted;
    for (const [typed, headline, copy, rows] of subnets) {
        asked++;
        const got = Subnets.answer(typed);
        if (!got || got.headline !== headline || got.copy !== copy
                || quoted(parted(got.rows)) !== quoted(rows))
            failures.push(`"${typed}" told ${quoted(got && [got.headline, got.copy,
                parted(got.rows)])}, not ${quoted([headline, copy, rows])}`);
    }
    for (const typed of notSubnets) {
        asked++;
        const got = Subnets.answer(typed);
        if (got)
            failures.push(`"${typed}" told ${quoted(got.headline)}, but nothing should`);
    }
    // A split into more than it lists says how many it lists.
    const split = Subnets.answer("10.0.0.0/8 in /24s");
    asked++;
    if (!split || split.headline !== "65,536 /24s" || split.rows.length !== 256
            || split.note !== "the first 256")
        failures.push(`"10.0.0.0/8 in /24s" told ${quoted(split && [split.headline,
            split.rows.length, split.note])}`);
    return asked;
}
