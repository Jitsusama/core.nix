// pi at a glance: how many sessions run and how many are working, for the
// launcher's foot, where the time and the battery are. There's nothing to
// show with no pi running, and nothing it wants of Joel, so it has no mark.
import qs
import QtQuick

Piece {
    present: Sessions.running.length > 0
    value: Sessions.working > 0
        ? `${Sessions.running.length} running, ${Sessions.working} working`
        : `${Sessions.running.length} running`
    title: "pi"
}
