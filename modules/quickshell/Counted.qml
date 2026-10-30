// Counts the frames of the window it sits in, by its name, while Frames
// counts. It's an empty item in the window's contents, since QtQuick's
// attached `Window.window` is the one way QML reaches the QtQuick window
// behind one of Quickshell's, and it follows the window when Quickshell
// makes it again, as it does for another screen.
import qs
import QtQuick

Item {
    id: counted

    required property string name

    Connections {
        target: counted.Window.window
        enabled: Frames.counting

        function onFrameSwapped() {
            Frames.heard(counted.name);
        }
    }
}
