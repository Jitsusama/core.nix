pragma Singleton
// How the microphones' noise is cancelled, and Choose by Ear's trial, as
// the noise-cancelled daemon keeps them: its one `trial:now` record, read
// over $XDG_RUNTIME_DIR/daemons/noise-cancelled.sock through Daemon.qml,
// as every daemon is held, and told whole on each change. The trial's
// fields and acts are the trial protocol's (QEST-20261111-JCWNFD's
// trial-protocol.md): its stage, the clips, the one playing, the pick and
// what each cleaner costs, and the cleaner the microphones run now.
// The launcher's Sound draws only from here (launcher/Sound.qml), and asks
// each act over the same connection.
import Quickshell
import QtQuick
import "launcher/ByEar.js" as ByEar

Singleton {
    id: root

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/noise-cancelled.sock`
        kind: "trial"
    }

    // The trial as the daemon says it, or null until it does, or when what
    // it says isn't one.
    readonly property var record: daemon.revision >= 0
        ? ByEar.whole(daemon.held["trial:now"] ?? null) : null
    // How the daemon stands, in Daemon.qml's words: "starting", its own,
    // or "not answering".
    readonly property string state: daemon.state
    // Said once a change, so what draws the trial draws it again.
    signal heard

    Connections {
        target: daemon

        function onHeard() {
            root.heard();
        }
    }

    // Starts the daemon while it isn't answering: its socket starts it on
    // the next ask, so connecting again at once is the asking.
    function start() {
        daemon.connectAgain();
    }

    // Asks one of the trial's acts, as ByEar.asked words it; `then` is
    // given "" once it's taken, the daemon's plain reason when it refuses,
    // or null when the daemon isn't there to ask. What follows is the
    // record's to say.
    function act(said, then) {
        daemon.ask(said, answer => {
            if (!then)
                return;
            if (answer === null)
                then(null);
            else
                then(answer.error ? String(answer.error.message ?? answer.error.code ?? "") : "");
        });
    }
}
