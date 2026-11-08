// What the launcher's WARP makes of what `warp --json` says (modules/warp,
// whose warp.json is the record's shape): how the link stands, in a few
// words for its row in Network, and each figure as a part, in its own
// unit. Warp.qml runs warp and draws the rows; nothing here asks anything,
// so tests/answers.qml runs it in Qt's own engine over warp.json itself.
.pragma library

const AND = " \u00b7 ";
const PERCENT = 100;
const TENS = 10;
const LOSS_PLACES = 2;
const SIZES = [
    [1e12, "TB"],
    [1e9, "GB"],
    [1e6, "MB"],
    [1e3, "kB"],
];
// warp-cli's reasons that say how the network is, "Network" then a word.
const NETWORK = /^Network([A-Z].*)$/;
// A protocol given with how it's carried: "MASQUE (HTTPS via UDP)".
const CARRIED = /^(.*\S)\s*\((.+)\)$/;
const CAPITAL = /([a-z0-9])([A-Z])/g;
const UNDERSCORE = /_/g;
// The unit a figure's key ends in, as warp-cli names them.
const UNIT = /^(.*)_(percent|ms)$/;
// What warp says when it couldn't read WARP, as its row says it; a
// machine with no warp-cli has no WARP, so says nothing.
const FAILED = {
    "not-answering": { headline: "not answering", note: (said) => `WARP's service didn't answer: ${said}` },
    unreadable: { headline: "unreadable", note: (said) => said },
};

// warp-cli's word as a person says it: "NetworkHealthy" as "network
// healthy".
function spoken(word) {
    return String(word).replace(CAPITAL, "$1 $2").replace(UNDERSCORE, " ").toLowerCase();
}

// Bytes in the powers of ten a network counts in, as Machine's are said.
function bytes(count, number) {
    const size = SIZES.find((each) => count >= each[0]);
    if (!size) {
        return `${Math.round(count)} B`;
    }
    const scaled = count / size[0];
    return `${number(scaled, scaled < TENS ? 1 : 0)} ${size[1]}`;
}

// What a reason's figures say, by the unit each key ends in:
// { loss_percent: 5.37, rtt_ms: 170 } as "5.37% loss, 170 ms round trip".
function measured(figures, number) {
    return Object.keys(figures ?? {})
        .map((key) => {
            const unit = UNIT.exec(key);
            const value = Number(figures[key]);
            if (!(unit && Number.isFinite(value))) {
                return "";
            }
            const name = unit[1] === "rtt" ? "round trip" : spoken(unit[1]);
            return unit[2] === "percent"
                ? `${number(value, LOSS_PLACES)}% ${name}`
                : `${number(value, 0)} ms ${name}`;
        })
        .filter((said) => said !== "")
        .join(", ");
}

// A part saying a figure, copying it as given, warning of nothing unless
// it's said to.
function part(id, side, note, copy) {
    return { id, side, note: note ?? "", copy: copy ?? side, warns: false };
}

// The network's health as WARP judges it, from a reason that says so, or
// null: healthy, or degraded with what WARP measured it by.
function health(status, number) {
    const judged = NETWORK.exec(status.reason ?? "");
    if (!judged) {
        return null;
    }
    const side = spoken(judged[1]);
    const figures = measured(status.reason_figures, number);
    const note = figures === "" ? "" : `${figures}, as WARP measured it`;
    return Object.assign(part("health", side, note), { warns: side !== "healthy" });
}

// The tunnel's parts, each only where warp gave its figure.
function tunnelParts(tunnel, number) {
    const parts = [];
    const estimated = "as WARP estimates it";
    const through = "through the tunnel";
    if (Number.isFinite(tunnel.latency_ms)) {
        parts.push(part("latency", `${number(tunnel.latency_ms, 0)} ms`, estimated,
            String(tunnel.latency_ms)));
    }
    if (Number.isFinite(tunnel.loss_fraction)) {
        parts.push(part("loss", `${number(tunnel.loss_fraction * PERCENT, LOSS_PLACES)}%`,
            estimated, String(tunnel.loss_fraction)));
    }
    if (typeof tunnel.protocol === "string" && tunnel.protocol !== "") {
        const carried = CARRIED.exec(tunnel.protocol);
        parts.push(carried
            ? part("protocol", carried[1], carried[2], tunnel.protocol)
            : part("protocol", tunnel.protocol));
    }
    const colo = tunnel.edge?.colo ?? "";
    if (colo !== "") {
        const metal = tunnel.edge.metal ?? "";
        parts.push(part("colo", colo, metal === "" ? "" : `metal ${metal}`));
    }
    for (const [id, key] of [["sent", "sent_bytes"], ["received", "received_bytes"]]) {
        if (Number.isFinite(tunnel[key])) {
            parts.push(part(id, bytes(tunnel[key], number), through, String(tunnel[key])));
        }
    }
    return parts;
}

// How WARP stands, from what `warp --json` printed: { headline, warns,
// parts }, each part { id, side, note, copy, warns } by its id in the
// vocabulary (warp:<id>), or null for a record with no status. The
// headline is its state, or the network's health while connected, with
// the latency beside it where warp gave one: "connected · 127 ms".
function told(record, number) {
    const status = record?.status;
    if (!status || typeof status.state !== "string") {
        return null;
    }
    const state = spoken(status.state);
    const judged = health(status, number);
    // A reason that isn't the network's health says why it stands so.
    const why = !judged && status.reason ? spoken(status.reason) : "";
    const parts = [part("state", state, why)];
    if (judged) {
        parts.push(judged);
    }
    const tunnel = record.tunnel ?? null;
    if (tunnel) {
        parts.push(...tunnelParts(tunnel, number));
    }
    const warns = judged?.warns === true;
    const first = warns ? judged.side : state;
    const latency = parts.find((each) => each.id === "latency");
    return {
        headline: latency ? `${first}${AND}${latency.side}` : first,
        warns,
        parts,
    };
}

// What warp said when it couldn't read WARP ({ error, said }), as the row
// says it: { headline, warns, note }, or null where there's no warp-cli,
// so no WARP to show.
function failed(printed) {
    const said = FAILED[printed?.error];
    if (!said) {
        return null;
    }
    return { headline: said.headline, warns: true, note: said.note(String(printed.said ?? "")) };
}
