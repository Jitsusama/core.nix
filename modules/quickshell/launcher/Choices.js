// The answers a session offers Joel to the one question it asks him, out
// of its want as its presence says it (harness #1394): `ask: { choices:
// [{ value, label, description? }], recommended?, allowOther? }` on a
// single question; a tabbed ask has none, and is dealt with in the
// session itself. For You lists them where he finds the ask
// (launcher/Now.qml's answerRow); kept apart so tests/now-moment.qml runs
// it through Now.
.pragma library

// What a want offers: null when it offers no answers, else the want's id
// as the session knows it, `<kind>@<since>`, its answers with the one it
// recommends first and the rest in its order, each { value, label,
// description, recommended }, and whether it takes Joel's own words.
function of(wants) {
    const ask = wants?.ask;
    const since = Date.parse(wants?.since ?? "");
    if (!ask || !Array.isArray(ask.choices) || Number.isNaN(since))
        return null;
    const recommended = String(ask.recommended ?? "");
    const choices = ask.choices
        .filter(one => typeof one?.value === "string" && one.value !== "")
        .map(one => ({
            value: one.value,
            label: String(one.label || one.value),
            description: String(one.description ?? ""),
            recommended: recommended !== "" && one.value === recommended
        }));
    if (choices.length === 0)
        return null;
    return {
        want: `${String(wants.kind ?? "")}@${new Date(since).toISOString()}`,
        choices: choices.filter(one => one.recommended)
            .concat(choices.filter(one => !one.recommended)),
        own: ask.allowOther === true
    };
}
