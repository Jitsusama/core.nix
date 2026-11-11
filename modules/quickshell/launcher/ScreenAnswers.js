.pragma library
// What a screen's power says before anything is asked, on the launcher's
// line, in plain JavaScript so tests/screen-answers.qml runs it in Qt's
// own engine, as the shell does. What niri answers a change, refused or
// to a screen gone, the windows daemon says (acts.rs), since it asks
// niri. `words` is Displays' screenWords.

// A refusal's words with what wasn't done and why filled in, each given
// by a function, so a $ in the words reads as written rather than as
// replace()'s pattern.
function filled(form, thing, why) {
    return form.replace("{thing}", () => thing).replace("{why}", () => why);
}

// What a screen's power says where Screens.toldOf tells it without
// asking niri: how the screen already stands, or that the last lit one
// isn't turned off or unplugged, in the same form as niri's refusals.
function toldSaid(told, named, words) {
    if (told === "not-off" || told === "not-unplugged") {
        return filled(words.said("", told), named, words.said("", "the-last-lit"));
    }
    return words.said("", told).replace("{name}", () => named);
}
