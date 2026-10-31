// The AI tokens running out: when the account's token limit will be
// reached at the last hour's rate, or that it was and when it's back, as
// spend-watch (modules/spend-watch) says it in the `tokens` of its status
// file. It's there only while that warns, early enough to slow down, so
// it's one thing, said once as a notice as it comes (Arrivals.qml) and
// listed in Now among what wants Joel while it holds, both from the same
// words: the notice's summary is the row's title, word for word, and its
// message the row's side and note. It's one card ever: its words follow
// the minutes in place, and as the daemon's `level` steps down the same
// card is raised again, so it never repeats (VKCF3Z, 2026-10-31). The daemon writes every word ready to draw, as for the
// spend piece, and the quiet ones, which hold no figures, while the screen
// is shared. It reads the same file as that piece, which costs a second
// small read each time the daemon writes, about every 30 seconds.
import qs
import Quickshell
import QtQuick

Piece {
    id: piece

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property var tokens: status.said?.tokens ?? null
    readonly property var shown: tokens === null ? null
        : Quiet.sharing ? tokens.quiet ?? null : tokens
    // Figures that can't be read just now are said as such, never guessed.
    readonly property string unread: shown?.unread ?? ""

    // Only while the daemon runs, since a stopped one's figures are old; the
    // spend piece says then that nothing watches.
    present: shown !== null && !status.silent
    active: present
    wants: active
    // Minutes from running out, or out, it needs him now.
    now: tokens?.now === true
    role: now ? "alert" : "warning"
    title: shown?.title ?? ""
    value: shown?.value ?? ""
    hint: title
    context: unread !== "" ? unread : shown?.context ?? ""
    message: [value, context].filter(part => part !== "").join(" \u00b7 ")

    arrives: true
    withdraws: true
    telling: tokens?.level ?? ""
    app: "Tokens"

    // See What Spends opens the spend collection, which the daemon sorts by
    // the last hour while this warns; Pause pauses the session spending
    // most, by pi's id for it, as Agents pauses one, wherever it runs,
    // named as Agents names the session rather than by an id Joel would
    // have to read, and left out when no running session has that id.
    // There's no Stop:
    // stopping loses work, which isn't the way to save tokens, and Pause
    // is undone with Resume. An act of a kind this doesn't know is left out.
    actions: (tokens?.actions ?? []).map(action => {
        if (action.kind === "spends")
            return {
                glyph: Theme.glyph.trend,
                name: action.name,
                run: shell => shell.launcher.into("spend")
            };
        const session = action.kind === "pause"
            ? Sessions.running.find(one => one.presence.session === action.session) : undefined;
        return session === undefined ? null : {
            glyph: Words.glyph("verb:agent:pause"),
            name: `${Words.name("verb:agent:pause")} ${session.title}`,
            run: () => Sessions.steer(session.address, { act: "pause" })
        };
    }).filter(action => action !== null)

    // The status the daemon wrote, or nothing when it wrote none that reads
    // or one of a version this doesn't know.
    function parsed(text) {
        try {
            const status = JSON.parse(text);
            return typeof status === "object" && status !== null && status.v === 1
                ? status : null;
        } catch (error) {
            return null;
        }
    }

    StatusFile {
        id: status
        path: piece.settings.status ?? `${piece.runtime}/spend-watch/status.json`
        silence: piece.settings.silence ?? 120
        parse: text => piece.parsed(text)
    }
}
