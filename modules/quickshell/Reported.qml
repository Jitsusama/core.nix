// Reports one surface for the desktop test (Report.qml), from inside its
// window: "drawn <name>" once a frame of it has been handed to niri after
// it's shown or something in it changed, "keys <name>" when niri gives it
// the keys, and "gone <name>" when it's hidden. A surface that shows
// something new while it stays up calls changed(), so its next frame is
// said too. With reports off it listens to nothing.
import QtQuick

Item {
    id: reported

    required property string name

    // Whether the next frame is one to say.
    property bool due: true
    readonly property var surface: Options.reports ? Window.window : null
    readonly property bool shown: surface !== null && Window.visibility !== Window.Hidden
    readonly property bool keys: surface !== null && Window.active

    function changed() {
        due = true;
    }

    onSurfaceChanged: due = true
    onShownChanged: {
        if (shown)
            due = true;
        else if (Options.reports)
            Report.say(`gone ${name}`);
    }
    onKeysChanged: {
        if (keys)
            Report.say(`keys ${name}`);
    }

    Connections {
        target: reported.surface

        function onFrameSwapped() {
            if (!reported.due || !reported.shown)
                return;
            reported.due = false;
            Report.say(`drawn ${reported.name}`);
        }
    }
}
