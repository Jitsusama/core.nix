// What virsh says of the VMs on a connection, as Machines.qml reads it
// and Programs › VMs shows it: the machines a list names, the batch that
// describes them, every figure dominfo gives for each and why any it
// leaves out is missing, the word for each state, what can be done in
// each state, and the order and count Programs shows them in. Kept apart
// from the QML so Qt's own engine runs it (tests/machines.qml).
//
// virsh is run with LC_ALL=C, so its labels and states are always these
// English ones, which gettext would otherwise translate.
.pragma library
.import "HomeNetwork.js" as Home

// A number keeps to its unit with a no-break space, so "12 s" never
// leaves its s alone on the next line.
const KEPT = "\u00a0";
const BETWEEN = " \u00b7 ";
// dominfo writes each label padded to 15 columns, a space, then the
// value; a label longer than that runs on with one space.
const COLUMN = 16;
// libvirt counts memory in KiB, each 1024 bytes.
const BYTES_IN_A_KIB = 1024;
const SECONDS_IN_A_MINUTE = 60;
const MINUTES_IN_AN_HOUR = 60;
const HOURS_IN_A_DAY = 24;
const SECONDS_IN_AN_HOUR = MINUTES_IN_AN_HOUR * SECONDS_IN_A_MINUTE;
// Processor time is said in two units at most, largest first.
const SPANS = [["d", HOURS_IN_A_DAY * SECONDS_IN_AN_HOUR], ["h", SECONDS_IN_AN_HOUR],
    ["min", SECONDS_IN_A_MINUTE], ["s", 1]];
// What dominfo writes a count, memory and processor time as: "4", "2097152
// KiB" and "12.3s"; and a line that carries on the one before, indented.
const WHOLE = /^\d+$/;
const KIB = /^(\d+) KiB$/;
const SECONDS = /^([\d.]+)s$/;
const INDENTED = /^\s/;

// The machines `virsh list --all --name` names, one a line, the blank line
// virsh ends the list with left out. libvirt refuses a name holding a
// newline, so a line is a whole name, and it's kept as it is, since a
// space at either end is part of it.
function listed(text) {
    return String(text ?? "").split("\n").filter(line => line.trim() !== "");
}

// Every character but a letter or a digit, which virsh's own command line
// would read as a quote, a space or the semicolon between commands.
const SPECIAL = /[^A-Za-z0-9]/g;

// A name as virsh's command line reads it back whole: each character that
// could mean something there behind a backslash, which virsh takes as the
// character itself. Given after --domain, a name starting with a hyphen
// is still read as the name rather than an option.
function quoted(name) {
    return String(name).replace(SPECIAL, character => `\\${character}`);
}

// The one line that asks dominfo of every machine named, so virsh connects
// once for them all: "dominfo --domain web; dominfo --domain build\ box".
function describing(names) {
    return names.map(name => `dominfo --domain ${quoted(name)}`).join("; ");
}

// A machine before dominfo has said anything of it: every figure unknown.
function blank() {
    return {
        id: null,
        name: "",
        uuid: "",
        state: "",
        processors: null,
        processorTime: null,
        mostMemory: null,
        noLimit: false,
        memory: null,
        persistent: null,
        autostart: null,
        managedSave: null,
        securityModel: "",
        securityLabel: "",
        messages: []
    };
}

// What a yes or no dominfo gives means: its Persistent and Managed save say
// yes, no or unknown, and its Autostart enable, disable or no autostart,
// that last where libvirt couldn't say.
const YES = { yes: true, enable: true, no: false, disable: false };

function answered(value) {
    return YES[value] ?? null;
}

// A whole number, as dominfo writes an Id or a count, or null for its "-".
function whole(value) {
    return WHOLE.test(value) ? Number(value) : null;
}

// A count of KiB, as dominfo writes memory: "2097152 KiB".
function kib(value) {
    const found = KIB.exec(value);
    return found ? Number(found[1]) : null;
}

// Seconds to a tenth, as dominfo writes processor time: "12.3s".
function seconds(value) {
    const found = SECONDS.exec(value);
    return found ? Number(found[1]) : null;
}

// What each of dominfo's labels sets, given its value with the spaces
// around it taken off; the name is set apart, as it's kept whole. Its OS
// Type is left out, since it says only how libvirt runs it ("hvm"), and
// its Security DOI, the labelling domain its model's labels are read in
// ("0"), since both are machinery (VKCF3Z, 2026-11-08 and 11-09).
const TAKES = {
    "Id:": (machine, value) => { machine.id = whole(value); },
    "UUID:": (machine, value) => { machine.uuid = value; },
    "State:": (machine, value) => { machine.state = value; },
    "CPU(s):": (machine, value) => { machine.processors = whole(value); },
    "CPU time:": (machine, value) => { machine.processorTime = seconds(value); },
    "Max memory:": (machine, value) => {
        machine.noLimit = value === "no limit";
        machine.mostMemory = kib(value);
    },
    "Used memory:": (machine, value) => { machine.memory = kib(value); },
    "Persistent:": (machine, value) => { machine.persistent = answered(value); },
    "Autostart:": (machine, value) => { machine.autostart = answered(value); },
    "Managed save:": (machine, value) => { machine.managedSave = answered(value); },
    "Security model:": (machine, value) => { machine.securityModel = value; },
    "Security label:": (machine, value) => { machine.securityLabel = value; },
    "Messages:": (machine, value) => { machine.messages.push(value); }
};

// One of dominfo's lines, as { label, value }: its label up to the colon,
// "" for a line that carries on the one before, as its Messages do, which
// starts where a label would, under the spaces, and may hold a colon of
// its own ("tainted: custom-monitor").
function parted(line) {
    const colon = line.indexOf(":");
    if (colon < 0 || INDENTED.test(line)) {
        return { label: "", value: line.trim() };
    }
    return {
        label: line.slice(0, colon + 1).trim(),
        value: line.slice(Math.max(COLUMN, colon + 2))
    };
}

// Each machine `virsh "dominfo A; dominfo B"` describes, in the order it
// says them: a new one begins at each Id. virsh goes on past one it can't
// find, as a machine undefined between the list and this, which it says
// only on its error stream, so that one is left out. A label a later
// libvirt adds is left out until it's read here, rather than shown under
// a name nobody chose.
function described(text) {
    const machines = [];
    let last = "";
    for (const line of String(text ?? "").split("\n")) {
        last = taken(machines, line, last);
    }
    return machines.filter(machine => machine.uuid !== "");
}

// One of dominfo's lines taken into the machines read so far, `last` the
// label the line before it gave, which a line carrying on carries on;
// gives the label it leaves for the next.
function taken(machines, line, last) {
    const { label, value } = parted(line);
    if (label === "Id:") {
        machines.push(blank());
    }
    const machine = machines[machines.length - 1];
    if (!machine) {
        return last;
    }
    if (label === "") {
        if (value !== "" && last === "Messages:") {
            machine.messages.push(value);
        }
        return last;
    }
    if (label === "Name:") {
        machine.name = value;
    } else if (TAKES[label]) {
        TAKES[label](machine, value.trim());
    }
    return label;
}

// Each state virsh says, by the launcher's own word for it, which
// words.json's vm entry's `said` spells, never libvirt's (VKCF3Z,
// 2026-11-08): idle is libvirt's blocked, on but waiting on what it asked
// for, in shutdown is powering off, as Power It Off does, shut off is
// off, and pmsuspended is the guest asleep, its memory kept, as the
// glossary's sleep is. Anything else, as virsh's no state, is unknown.
const STATES = {
    "running": "running",
    "idle": "waiting",
    "paused": "paused",
    "in shutdown": "stopping",
    "shut off": "off",
    "crashed": "crashed",
    "pmsuspended": "asleep"
};

// Every state a machine can be in here, in the order Programs' note
// counts them: those it's on in first, then the rest.
const ORDER = ["running", "waiting", "paused", "stopping", "crashed", "asleep", "unknown", "off"];

function stateOf(said) {
    return STATES[String(said ?? "").trim()] ?? "unknown";
}

// What a machine says beside it in a state, as words.json says it.
function side(state, said) {
    const words = said ?? {};
    return words[state] ?? words.unknown ?? "";
}

// What can be done to a machine in each state, in the order it's listed:
// its console first, since opening it changes nothing, while there's a
// screen to see; then starting it, resuming it or powering it off; and
// forcing it off last, since it ends whatever it was doing. A crashed one
// libvirt kept is still active, so it can't start until it's forced off,
// and one asleep wakes from its own console, which virt-manager does.
// Nothing is offered in a state virsh can't name.
const ACTS = {
    running: ["console", "power-off", "force-off"],
    waiting: ["console", "power-off", "force-off"],
    paused: ["console", "resume", "force-off"],
    stopping: ["console", "force-off"],
    off: ["start"],
    crashed: ["console", "force-off"],
    asleep: ["console", "force-off"],
    unknown: []
};

function acts(state) {
    return ACTS[state] ?? [];
}

// What virsh is told to do for each act, given the machine's UUID, which
// names it whatever its name holds, and which no name starting with a
// hyphen can be read as an option in place of. Pausing is only how u
// takes back Resume It, so it has no row of its own.
const VIRSH = { "start": "start", "resume": "resume", "pause": "suspend",
    "power-off": "shutdown", "force-off": "destroy" };

function told(act, uuid) {
    return VIRSH[act] ? [VIRSH[act], "--domain", uuid] : [];
}

// The state an act leaves a machine in, or "" for powering it off, which
// the guest does in its own time, and may not do at all.
const BECOMES = { "start": "running", "resume": "running", "pause": "paused",
    "force-off": "off" };

function becomes(act) {
    return BECOMES[act] ?? "";
}

// Whether a line `virsh event --loop --event lifecycle` prints says a
// machine changed: "event 'lifecycle' for domain 'web': Started Booted".
// It names the machine, but every change is read whole, so a machine
// defined or undefined is found as one started is.
function changed(line) {
    return String(line ?? "").startsWith("event 'lifecycle' ");
}

// What virsh said went wrong, its last error line, as the launcher's foot
// says it, lowercase after its first word: "requested operation is not
// valid: domain is already active". "" where it said none.
function reason(text) {
    const errors = String(text ?? "").split("\n").map(line => line.trim())
        .filter(line => line.startsWith("error: "));
    if (errors.length === 0) {
        return "";
    }
    const said = errors[errors.length - 1].slice("error: ".length).trim();
    return said.charAt(0).toLowerCase() + said.slice(1);
}

// A name's runs of digits and of everything else, so "dev 9" comes before
// "dev 10": Qt's engine passes over localeCompare's numeric option.
const RUNS = /\d+|\D+/g;
const DIGITS = /^\d/;

function natural(one, other) {
    const ones = one.toLowerCase().match(RUNS) ?? [];
    const others = other.toLowerCase().match(RUNS) ?? [];
    for (let at = 0; at < Math.min(ones.length, others.length); at += 1) {
        const numbers = DIGITS.test(ones[at]) && DIGITS.test(others[at]);
        const by = numbers ? Number(ones[at]) - Number(others[at])
            : ones[at].localeCompare(others[at]);
        if (by !== 0) {
            return by;
        }
    }
    return ones.length - others.length;
}

// The machines as Programs lists them: those on before those off, as
// Programs puts open programs first, then each A to Z, a session one
// before a system one of the same name.
function ordered(machines) {
    const off = one => Number(stateOf(one.state) === "off");
    return machines.slice().sort((one, other) => off(one) - off(other)
        || natural(one.name, other.name)
        || Number(one.system === true) - Number(other.system === true));
}

// How many are in each state, as the program's row says it below its
// name: "1 running · 2 off", each state in ORDER.
function tally(machines, said) {
    const counts = {};
    for (const one of machines) {
        const state = stateOf(one.state);
        counts[state] = (counts[state] ?? 0) + 1;
    }
    const each = said?.count ?? "{count} {state}";
    return ORDER.filter(state => counts[state] > 0)
        .map(state => each.replace("{count}", String(counts[state]))
            .replace("{state}", side(state, said)))
        .join(BETWEEN);
}

// One place after the point, and none where it would be a zero.
function places(value) {
    const fixed = value.toFixed(1);
    return fixed.endsWith(".0") ? fixed.slice(0, -2) : fixed;
}

// Memory, which libvirt counts in KiB, said in decimal units from its
// bytes, as the launcher says every size (VKCF3Z, 2026-11-08), through
// the shell's own formatter: 4194304 KiB as "4.29 GB", 524288 KiB as
// "537 MB".
function memory(count) {
    if (count === null || count === undefined) {
        return "";
    }
    return Home.bytes(count * BYTES_IN_A_KIB);
}

// Processor time, which dominfo gives in seconds to a tenth: "12.3 s",
// "4 min 12 s", "3 h 5 min", "2 d 4 h", two units at most, so a long run
// still reads at a glance.
function duration(time) {
    if (time === null || time === undefined) {
        return "";
    }
    if (time < SECONDS_IN_A_MINUTE) {
        return `${places(time)}${KEPT}s`;
    }
    const all = Math.floor(time);
    const first = SPANS.findIndex(span => all >= span[1]);
    const [big, size] = SPANS[first];
    const [small, step] = SPANS[first + 1];
    const most = `${Math.floor(all / size)}${KEPT}${big}`;
    const rest = Math.floor((all % size) / step);
    return rest > 0 ? `${most} ${rest}${KEPT}${small}` : most;
}

// A yes or no as words.json says it, or "" where libvirt couldn't say.
function yesOrNo(value, said) {
    if (value === null) {
        return "";
    }
    return value ? said.yes ?? "yes" : said.no ?? "no";
}

// Every figure dominfo can give for a machine, each [id, value], its id
// words.json's `vm:<id>` naming it, "" where it gave none, in the order a
// plate runs (docs/design.md, the drawer): what moves most, its processor
// time and memory in use, then what it's given to run with, whether it
// starts by itself and keeps its definition, a saved state where it has
// one, how it's kept apart from the machine where it is, and what names it
// last. A model of "none" says nothing, as a saved state that isn't does
// (VKCF3Z, 2026-11-09), so a drawer holds every figure without a "more".
// Its name is the plate's title and its state the row's side, so neither
// is said again. A saved state is said only where there is one, since
// most never have one and "no" would take a line for nothing (VKCF3Z,
// 2026-11-08).
// Memory in use is said only while it's on: dominfo gives one off too,
// but that's only what it would start with (VKCF3Z, 2026-11-08).
function everyFigure(machine, said) {
    const words = said ?? {};
    const most = machine.noLimit ? words.noLimit ?? "no limit" : memory(machine.mostMemory);
    const off = stateOf(machine.state) === "off";
    return [
        ["processor-time", duration(machine.processorTime)],
        ["memory", off ? "" : memory(machine.memory)],
        ["processors", machine.processors === null ? "" : String(machine.processors)],
        ["most-memory", most],
        ["autostart", yesOrNo(machine.autostart, words)],
        ["persistent", yesOrNo(machine.persistent, words)],
        ["managed-save", machine.managedSave === true ? words.yes ?? "yes" : ""],
        ["security-model", machine.securityModel === "none" ? "" : machine.securityModel],
        ["security-label", machine.securityLabel],
        ["messages", machine.messages.join("; ")],
        ["id", machine.id === null ? "" : String(machine.id)],
        ["uuid", machine.uuid]
    ];
}

// The figures that name a machine rather than measure it, its UUID and
// the label its security model gave it, which its drawer sets as
// identifiers, whole on as many lines as they take, breaking at their
// hyphens, rather than one word running past the drawer's edge.
const IDENTIFIERS = ["uuid", "security-label"];

// A figure as its drawer's fact, `label` its name: one naming a machine
// (IDENTIFIERS) is an identifier, one whole item the drawer lays on as
// many lines as it needs, broken at its own marks (Identifier.js), as a
// program's command is; any other, its value alone.
function factOf(figure, label) {
    const fact = { label, value: figure.value };
    return IDENTIFIERS.includes(figure.id)
        ? Object.assign(fact, { valueItems: [{ said: figure.value, whole: true }],
            between: " ", identifies: true })
        : fact;
}

// Every figure dominfo gave for a machine, each { id, value }, in the
// plate's order. One it left out is left out, never guessed, and named by
// leftOut instead.
function figures(machine, said) {
    return everyFigure(machine, said).filter(one => one[1] !== "")
        .map(one => ({ id: one[0], value: one[1] }));
}

// The figures only a running machine has: its Id, the processor time and
// memory it's using and the label its security model gave this run, all
// of which it gives up as it stops.
const WHILE_RUNNING = ["processor-time", "memory", "security-label", "id"];

// The figures that are a machine's own whether or not it runs, which
// dominfo leaves out only where libvirt couldn't tell: an Autostart of
// "no autostart", a Managed save of "unknown". A model and its label
// come only with one, and messages only when there's something to say,
// so none of those is missing when it's absent.
const ALWAYS = ["processors", "most-memory", "autostart", "persistent",
    "managed-save", "uuid"];

// The figures dominfo left out of a machine, by why, each a list of ids in
// the plate's order: `stopped`, those only a running machine has, while
// it's not running, and `unsaid`, those libvirt couldn't tell. A label is
// missing only where a model would give one, so a machine whose model is
// none, or that has none, never lacks it; and a saved state only where
// libvirt couldn't say whether there is one, not where there's none.
function leftOut(machine, said) {
    const off = stateOf(machine.state) === "off";
    const labelled = machine.securityModel !== "" && machine.securityModel !== "none";
    const missing = everyFigure(machine, said)
        .filter(one => one[1] === "" && (one[0] !== "managed-save" || machine.managedSave === null))
        .map(one => one[0]);
    return {
        stopped: missing.filter(id => off && WHILE_RUNNING.includes(id)
            && (id !== "security-label" || labelled)),
        unsaid: missing.filter(id => ALWAYS.includes(id)
            || (!off && WHILE_RUNNING.includes(id) && (id !== "security-label" || labelled)))
    };
}

// Names said as one list, "Processor Time, Security Label and ID", the
// last two joined by the launcher's word for and.
function together(names, and) {
    if (names.length < 2) {
        return names.join("");
    }
    return `${names.slice(0, -1).join(", ")} ${and} ${names[names.length - 1]}`;
}
