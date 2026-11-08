.pragma library
// What a pi session's drawer says, wherever the launcher shows one
// (Agents.qml), laid so its first screen holds what matters of it: what
// it's doing, what it needs Joel for, what it landed last, what it spent
// and its figures, then its quest and machine, and what its quest is about
// last of all.
// The drawer shows only whole facts, so a long one ahead of the rest left
// the title and "13 more" alone on every session (Joel, 2026-11-07: "the
// agents side panel generally just shows its title. i have to ctrl-w h
// into it and then scroll a ton of lines before i see any more details").

// The longest thing a step is on that's said beside its tool: anything
// longer is a command or a path the harness has already cut short.
const SHORT = 24;
const ELLIPSIS = "\u2026";
// A path, from the root or home, and all of it before its file's name.
const PATH = /^[~/]/;
const FOLDERS = /^.*\//;

// What a step is, from the record's `stepDetail`: "thinking", or a tool
// and what it's on, a file by its name alone.
function step(detail) {
    if (!detail?.kind) {
        return "";
    }
    if (detail.kind !== "tool") {
        return detail.kind;
    }
    const on = String(detail.detail ?? "");
    const whole = on !== "" && !on.endsWith(ELLIPSIS);
    const file = whole && PATH.test(on) ? on.replace(FOLDERS, "") : "";
    const said = file !== "" ? file : whole && on.length <= SHORT ? on : "";
    return [detail.name ?? "", said].filter(part => part !== "").join(" ");
}

// What its quest is about, from the record's `worksOn`, which leads with
// the quest's title, the drawer's own title already, then its summary.
function about(works, titles) {
    const text = String(works ?? "");
    const lead = titles.find(title => (title ?? "") !== "" && text.startsWith(`${title}: `));
    return lead ? text.slice(lead.length + 2) : text;
}

// Its address, where it works and since when it's run are the eyebrow over
// its title, "8IDU4E \u00b7 core.nix \u00b7 since 21:52" (VKCF3Z,
// 2026-11-06), and how it stands is its row's, on the same line, so none
// is said again. Then, each as a name and a value: what it's doing, its
// step alone, and nothing where it has none; what it needs Joel for (its
// want, then its quest's Needs You items, `said.needs`); what it landed
// last (`said.landed`); what it's spent today (an orchestrator's whole
// orchestration, then its own and its sessions'); the budget it drew on
// last and how that stands (`said.budget`, BudgetDrawer.js), after the
// spend it's part of; its figures
// (`said.figures`, SessionRow.figures); what rebuilding its context cost;
// its quest, how that stands said only where it isn't active; its
// machine; and what its quest is about, as words (VKCF3Z, 2026-11-07).
// Nothing it hasn't said is guessed. The eyebrow names things, so the
// drawer sets it as they're written, never in capitals (Drawer.qml).
function drawer(title, presence, said) {
    const quest = presence?.questInfo ?? null;
    const known = [
        ["step", step(presence?.stepDetail)],
        ["wants", [presence?.wants?.said ?? "", said.needs ?? ""]
            .filter(part => part !== "").join(" \u00b7 ")],
        ["last-landing", said.landed ?? ""],
        ["spent", said.spent ?? ""],
        ["own", said.own ?? ""],
        ["under", said.under ?? ""],
        ["budget", said.budget ?? ""]
    ];
    // Its own spend today carries its hours as Graph's spark (`said.hours`,
    // spend-watch's): an orchestrator's Its Own, any other's Spent Today.
    const hoursOn = (said.own ?? "") !== "" ? "own" : "spent";
    const facts = known.filter(([, value]) => value !== "")
        .map(([id, value]) => Object.assign({ label: said.named(id), value },
            id === hoursOn && said.hours ? { figure: said.hours } : {}));
    const stands = quest?.status && quest.status !== "active" ? quest.status : "";
    const task = quest?.id ? [quest.id, stands].filter(part => part !== "").join(" \u00b7 ")
        : "";
    const after = [
        ["rebuilt", said.rebuilt ?? ""],
        ["task", task],
        ["machine", said.machine ?? ""]
    ].filter(([, value]) => value !== "")
        .map(([id, value]) => ({ label: said.named(id), value }));
    const works = about(presence?.worksOn, [title, quest?.title]);
    const words = works === "" ? [] : [{ label: said.named("works"), value: works, words: true }];
    const eyebrow = [said.address, said.place, said.since]
        .filter(part => (part ?? "") !== "").join(" \u00b7 ");
    // Short of room, it gives way by whole items from its end, as the
    // held foot gives its position way whole: when it started goes first,
    // then where it works, and its address never (VKCF3Z, 2026-11-07:
    // "since 0\u2026" tells nothing).
    const eyebrowItems = [
        { said: said.address ?? "", keep: 3 },
        { said: said.place ?? "", keep: 2 },
        { said: said.since ?? "", keep: 1 }
    ];
    return {
        eyebrow, eyebrowItems, eyebrowNames: true, title,
        facts: facts.concat(said.figures ?? [], after, words),
        body: ""
    };
}
