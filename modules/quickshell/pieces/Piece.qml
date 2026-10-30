// The shape of one piece of a surface's content. A piece says what it shows
// and how things stand; the surface decides where and how it's drawn, so
// every piece, built in or a machine's own, lines up with the rest. A new
// piece is a file that starts from this one:
//
//     Piece {
//         value: "what it shows"
//         active: true // while it's true: tier "when" shows it, and its mark
//         role: "warning" // the theme's colour for it, by role
//     }
//
// Its settings from Nix, its format or its thresholds, are in `settings`.
//
// A piece with a mark is also drawn in the margin while it's true
// (Margin.qml), as its mark alone, placed and coloured by its `kind` and
// whether it's needed `now`. The launcher's What's on says it in words
// (launcher/WhatsOn.qml): its `title`, how it stands (`value`), what more
// there is to know (`context`), and what can be done (`actions`, launcher
// rows that run with the launcher as `shell.launcher`); `hint` is the
// sentence that says it whole. It `wants` Joel when he should act on it,
// and the launcher's foot then wears its kind's colour (`hue`), or its
// role's when that isn't plain text.
import QtQuick

Item {
    visible: false

    property var settings: ({})
    // Whether there's anything to show at all: no battery, no battery piece.
    property bool present: true
    property bool active: false
    property string value: ""
    property string role: "text"
    // The theme's glyph its value is read by at a glance, by name, or none,
    // and what a glance needs said after it: the least that tells Joel
    // something, or nothing when the glyph says it all. Where to look for
    // more is the piece's own collection.
    property string sign: settings.sign ?? ""
    property string glance: value
    // Its glance cut to the least of all, for a glance short of room.
    property string least: glance

    property string hint: value
    property string title: settings.label ?? ""
    property string context: ""
    property bool wants: false
    // How much it asks of Joel while it's true, which places its mark and
    // its colour: on air (`onAir`) while something of his reaches others,
    // a camera, a share, a recording, a call; `wants` while it needs him;
    // `leftOn` for a mode he set and might forget. And whether it needs
    // him now, not soon (`now`): the battery about to die, a call ringing.
    property string kind: wants ? "wants" : "leftOn"
    // Which of Now's groups it's listed in (launcher/Now.js): its kind,
    // but for one with a group of its own, as what's playing has.
    property string group: kind
    property bool now: false
    // The program whose notices say this piece's news, as a notification
    // names it, so its mark waits while one of its cards is on screen:
    // a card or a mark, never both.
    property string app: ""
    // Whether it comes true by itself, not by Joel's keys, and so is said
    // as a notice of its app's when it does (Arrivals.qml).
    property bool arrives: false
    // Whether that notice goes, read, once it's no longer true, as one of
    // a meeting about to start does once it has started: news only while
    // it lasts.
    property bool withdraws: false
    // What its notice tells, for one said more than once while it's true:
    // each time this changes, it's said again and its earlier notice, which
    // it `withdraws`, is taken back, as a meeting is at each of its
    // reminders and as it starts. Empty, it's said once.
    property string telling: ""
    // Whether another piece's mark stands for it, so the margin leaves its
    // own out while What's on still lists it: quiet turned on by a share is
    // the share's, whose words say notices are held.
    property bool collected: false
    // The glyph its mark is drawn with, by name, where it isn't the mark's
    // own: the power profile's mark says which profile is on. The mark
    // stays its name, so what Now lists is the same thing whichever it is.
    property string markGlyph: ""
    property string hue: "machine"
    // { glyph, name, chord, run(shell) }, and a `verb` where the name is
    // too long for the launcher's foot to say beside Return. One that only
    // changes how things stand, rather than taking Joel somewhere, says
    // `stays: true`, so the launcher stays open and shows the change
    // (Joel, 2026-10-26: Return closes only when it takes him out).
    property var actions: []
    // The things it stands for, where each is a thing of the launcher's
    // own, such as a piece of mail: given the launcher's `thing`, their
    // rows, which Return lists in place of doing its first act, so each
    // shows as it does everywhere else. Null for a piece that's only acts.
    property var rows: null
    // The things it stands for, each a fact of its own (Facts.js), where
    // it stands for several, as the Bluetooth devices' batteries do: each
    // a plain object read as a piece is, with a `key` of its own, made by
    // `thing`. Null for a piece that's one fact, as most are.
    property var each: null

    // A thing of `each`, as a piece of its own would read, from what it
    // says that a piece doesn't by default.
    function thing(own) {
        const wants = own.wants ?? false;
        const kind = own.kind ?? (wants ? "wants" : "leftOn");
        return Object.assign({
            settings: settings,
            present: true,
            active: false,
            value: "",
            role: "text",
            sign: "",
            glance: "",
            least: "",
            hint: "",
            title: "",
            context: "",
            wants: wants,
            kind: kind,
            group: kind,
            now: false,
            app: "",
            arrives: false,
            withdraws: false,
            telling: "",
            collected: false,
            markGlyph: "",
            // The launcher's place that lists it as a row of its own, by the
            // place's name, its row's key the thing's mark, so Now draws it
            // as that row rather than a second way (launcher/WhatsOn.qml's
            // own); "" for one that's only a fact.
            place: "",
            hue: "machine",
            actions: [],
            rows: null
        }, own);
    }
}
