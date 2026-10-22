pragma Singleton
// The desktop's facts: every overview piece (Overview.qml), loaded once from
// its file with its settings, for the two places that show them, the
// launcher's foot and the margin, so each fact is heard once, however many
// screens there are.
import Quickshell
import QtQuick
// Quickshell finds a folder's types only once a file it loads imports it,
// and the pieces are loaded later, by name.
import qs.pieces

Singleton {
    id: root

    // { entry, piece } in the overview's order, piece null until it loads.
    readonly property var all: {
        loaded.count;
        return Overview.pieces.map((entry, index) => {
            const loader = loaded.objectAt(index);
            return { entry: entry, piece: loader ? loader.item : null };
        });
    }

    Instantiator {
        id: loaded
        model: Overview.pieces

        Loader {
            required property var modelData
            Component.onCompleted: setSource("pieces/" + modelData.file, { settings: modelData })
        }
    }
}
