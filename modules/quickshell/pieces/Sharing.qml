// Another program seeing the screen, as a call shares it through the portal:
// true while niri says a PipeWire screencast streams, and what it sees, the
// whole screen or one window. niri's events say when, so nothing is asked.
// A window being recorded streams the same way, and is left to Recording.
import qs
import QtQuick

Piece {
    readonly property var casts: Object.values(Niri.casts)
        .filter(cast => cast.kind === "PipeWire" && cast.is_active
            && cast.session_id !== Recorder.windowSession)
    // A window's title as it is now: windows change in place, and the
    // revision says when.
    readonly property var seen: {
        Niri.revision;
        return casts.map(cast => cast.target.Window
            ? (Niri.windows[cast.target.Window.id] ?? { title: "a window" }).title
            : cast.target.Output ? "the whole screen" : "nothing yet");
    }

    active: casts.length > 0
    kind: "onAir"
    value: seen.length === 1 ? seen[0] : `${seen.length} things`
    title: "Sharing"
    hint: seen.length === 1 ? `Sharing ${seen[0]}` : `Sharing ${seen.length} things`
    context: casts.length === 1 ? Quiet.shared ? "notices are held" : "notices still show"
        : seen.join(", ")
    actions: [{
        glyph: Words.glyph("verb:on:sharing:stop"),
        name: Words.name("verb:on:sharing:stop", casts.length === 1 ? "" : "all"),
        verb: Words.verb("verb:on:sharing:stop"),
        stays: true,
        run: () => {
            for (const session of new Set(casts.map(cast => cast.session_id)))
                Niri.act({ StopCast: { session_id: session } });
        }
    }]
}
