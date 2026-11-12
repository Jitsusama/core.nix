// niri as Dictation.qml reaches it in the-desktop-follows-dictation: no
// windows and no casts, and nothing it's asked to spawn or do happens.
pragma Singleton
import Quickshell
import QtQuick
Singleton {
  property var windows: ({})
  property var casts: ({})
  function spawn(command) { }
  function act(request) { }
}
