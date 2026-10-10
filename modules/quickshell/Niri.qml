// The shell's one connection for telling niri what to do, over niri's own
// socket, so asking costs no process.
//
// Programs the shell opens are spawned by niri rather than by the shell.
// niri gives each one a systemd scope of its own; one the shell started
// itself would leave its helpers in the shell's service, where restarting
// the shell kills them. Google's Chrome wrapper pipes Chrome's output
// through two of those helpers, so a switch that restarted the shell took
// down every Chrome and Slack opened from the launcher.
pragma Singleton
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // Requests asked for before the socket connects, sent once it does.
    property var waiting: []

    function act(action: var): void {
        waiting.push(JSON.stringify({ Action: action }));
        if (socket.connected)
            send();
        else
            socket.connected = true;
    }

    function spawn(command: var): void {
        act({ Spawn: { command: command } });
    }

    function send(): void {
        for (const request of waiting)
            socket.write(request + "\n");
        waiting = [];
        socket.flush();
    }

    Socket {
        id: socket
        path: Quickshell.env("NIRI_SOCKET")
        connected: true
        onConnectionStateChanged: {
            if (connected && root.waiting.length > 0)
                root.send();
        }
        parser: SplitParser {
            onRead: line => {
                const reply = JSON.parse(line);
                if (reply.Err)
                    console.warn("niri refused a request:", reply.Err);
            }
        }
    }
}
