.pragma library
// What niri answers a change Displays asks of a screen, said on the
// launcher's warning line, in plain JavaScript so tests/screen-answers.qml
// runs it in Qt's own engine, as the shell does. niri refuses a change
// it can't take, as a mode of its own making it won't run or a request
// it can't read, and answers one to a screen gone before it came that
// the screen was missing; either way, before this, Return looked done
// and nothing said otherwise. `words` is Displays' screenWords.

// What closes niri's words for why: a full stop or several.
const CLOSING_STOP = /\.+$/;

// What wasn't done, by the word that says it, for a change niri
// refused: a setting set, a screen or a switch turned on or off, or a
// screen unplugged.
const NOT_DONE = { set: "not-set", on: "not-on", off: "not-off", unplug: "not-unplugged" };

// The warning line for a change niri refused, in the form a refusal on
// the line takes: what was asked, what wasn't done, and niri's own
// words for why, begun in lower case with no full stop closing them, as
// the foot's words are: "1.5\u00d7 not set \u00b7 invalid custom mode"
// (VKCF3Z, 2026-11-09). Only the first letter is lowered, so a screen's
// name niri gives in them, as eDP-1, keeps its own case. `refusal` is
// what the row did, as { thing, done }, `done` one of NOT_DONE's.
function refusalSaid(refusal, why, words) {
    const said = String(why).trim().replace(CLOSING_STOP, "");
    return words.said("", NOT_DONE[refusal.done]).replace("{thing}", refusal.thing)
        .replace("{why}", said.charAt(0).toLowerCase() + said.slice(1));
}

// What niri's answer to a screen's change says needs saying: nothing
// where it applied it, and where the screen named had gone before it
// came, that the screen is unplugged, named as its row is.
function answerSaid(answer, named, words) {
    return answer?.OutputConfigChanged === "OutputWasMissing"
        ? words.said("", "isUnplugged").replace("{name}", named) : "";
}
