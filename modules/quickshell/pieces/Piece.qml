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
// whether it's needed `now`. The launcher's Now says it in words
// (launcher/Now.qml): its `title`, how it stands (`value`), what more
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
    // Its value as it runs on by itself, given the time, for one that
    // counts faster than it's made again, as the stopwatch does to the
    // tenth: Now's row draws it afresh (Launcher.qml's counts). Null for
    // one whose value says it. And the widest that reads until it grows a
    // figure, the room Now keeps for it.
    property var counts: null
    property string widest: ""
    // Its value as items, `{ said, keep }`, where it says several, so Now's
    // row gives way by whole items (Choice.qml's sideItems); empty, the
    // value is said as it is.
    property var valueItems: []
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
    // What a thing's piece says after its title, by whole names giving
    // way from the end, as a call names who's in it: a thing's detail
    // follows its name.
    property var parts: []
    // What a piece that's a place holds, by name, as Timers does its
    // timers, for the drawer every place's wears; it says nothing after
    // its title (VKCF3Z, 5fa2cbda). Null for any other.
    property var heldNames: null
    // What its notice says as its summary, when it arrives: its hint,
    // unless the notice is of one thing among those the hint counts.
    property string summary: hint
    // What its notice says beneath its summary, when it arrives: its
    // context, unless it says more there.
    property string message: context
    property bool wants: false
    // How much it asks of Joel while it's true, which places its mark and
    // its colour: on air (`onAir`) while something of his reaches others,
    // a camera, a share, a recording, a call; `wants` while it needs him;
    // `leftOn` for a mode he set and might forget. And whether it needs
    // him now, not soon (`now`): the battery about to die, a call ringing.
    property string kind: wants ? "wants" : "leftOn"
    // Which of Now's groups it's listed in (launcher/NowGroups.js): its kind,
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
    // own out while Now still lists it: quiet turned on by a share is
    // the share's, whose words say notices are held.
    property bool collected: false
    // The glyph its mark is drawn with, by name, where it isn't the mark's
    // own: the power profile's mark says which profile is on. The mark
    // stays its name, so what Now lists is the same thing whichever it is.
    property string markGlyph: ""
    // The launcher's place pressing its mark opens, by the name the
    // launcher leads from (Launcher.qml's into), where the mark stands for
    // a place of its own rather than its row in Now, as For You's tray
    // does; "" opens Now on its row.
    property string opens: ""
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
    // Its second argument, `listed(parts, name, side)`, says parts after a
    // row's name in the room left, giving way by whole parts, as Now's own
    // row says the piece's `parts`.
    property var rows: null
    // What came for Joel that it stands for, where it's one of For You's
    // Lately rows (Now.qml) rather than a row of Now's own: its source, one
    // of launcher/Lately.js's SOURCES, and what it holds, each { who, at },
    // the newest first, by which the row is weighed and its note names
    // its members (VKCF3Z, 2026-11-08). Empty for every other piece.
    property string lately: ""
    property var came: []
    // Whether its source answers now, so it says by itself what's done
    // with, even with nothing left to read: Slack while its daemon does.
    // While it does, the notices its program sent stand for nothing in
    // For You (launcher/Lately.js's held).
    property bool answers: false
    // What it held that its source says is done with, for For You's
    // History, each with `done` { how, at }, its source's word for how and
    // when, and `row(thing)`, the row it is wherever it's shown; or null
    // while its source can't say, as Slack's daemon holding only some.
    property var finished: null
    // Everything its source says, as Now's row's drawer (Drawer.qml's
    // `drawer`), where the row can't show it all: given what the launcher
    // allows (`{ lookups }`, whether it may ask the network), the drawer.
    // Null for a piece whose row says all there is, or whose place's row
    // is its own and draws its own drawer.
    property var drawer: null
    // The launcher's row whose drawer is this piece's too, where a place
    // lists the thing it reads, as `{ place, key }`: the Microphone's is
    // its device's in Sound, the Camera's the camera in use's, Tokens' its
    // cap's in Spend (VKCF3Z,
    // 2026-11-08: one thing gets one drawer). Its own row and acts stay
    // its own. Null for a piece with no such row.
    property var drawnBy: null
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
            counts: null,
            widest: "",
            role: "text",
            sign: "",
            glance: "",
            least: "",
            hint: "",
            title: "",
            context: "",
            parts: [],
            message: own.context ?? "",
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
            // as that row rather than a second way (launcher/Now.qml's
            // own); "" for one that's only a fact.
            place: "",
            hue: "machine",
            actions: [],
            rows: null
        }, own);
    }
}
