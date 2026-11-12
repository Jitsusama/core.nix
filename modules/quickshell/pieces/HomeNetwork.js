// What ails the home network, worst first, as Now's Home Network piece
// says it (pieces/HomeNetwork.qml), from the record the eero daemon keeps
// (Eero.qml, modules/eero/src/model.rs) and in words.json's
// on:home-network words. Nothing ails it is an empty list, which leaves
// the piece quiet.
.pragma library

// A template said, its holes filled from `fields`.
function filled(template, fields) {
    const hole = /\{(?<name>\w+)\}/gu;
    return String(template ?? "").replace(hole, (whole, name) => fields[name] ?? whole);
}

// The nodes eero says are off line, by where they are.
function offline(model) {
    return (model?.nodes ?? [])
        .filter((node) => String(node.state ?? "").toUpperCase() === "OFFLINE")
        .map((node) => node.location || node.model || "");
}

// The nodes off line, named while one is and eero says where, else
// counted; eero's count stands where it lists fewer than it counts.
function nodesSaid(model, said) {
    const off = offline(model);
    const count = Math.max(off.length, model?.internet?.nodes?.offline ?? 0);
    if (count === 1 && off.length === 1 && off[0] !== "") {
        return filled(said.nodeOffline, { node: off[0] });
    }
    if (count === 1) {
        return said.oneOffline;
    }
    return count > 0 ? filled(said.nodesOffline, { count }) : "";
}

// While this machine is at home and its own look finds the internet out,
// that alone is said, since eero's cloud is then out of reach and all it
// last said of the internet is from before. While eero isn't answering
// otherwise, nothing is known now, and the place says why. Otherwise the
// internet, the provider, the nodes' network as a whole, then the nodes
// off line.
function ails(model, said) {
    const internet = model?.internet ?? {};
    const probe = internet.probe ?? null;
    if (probe?.home === true && probe.reachable === false) {
        return [said.internetDown];
    }
    if (model?.cloud?.state !== "answering") {
        return [];
    }
    const state = String(internet.state ?? "").toLowerCase();
    const mesh = String(internet.mesh ?? "").toLowerCase();
    return [
        state !== "" && state !== "connected" ? said.internetDown : "",
        internet.isp_up === false ? said.providerDown : "",
        mesh !== "" && mesh !== "connected" ? filled(said.mesh, { state: mesh }) : "",
        nodesSaid(model, said),
    ].filter((part) => part !== "");
}
