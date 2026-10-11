// The network: the Wi-Fi network's name, wired, or offline. It's true while
// the machine is offline, and takes the warning colour then.
import Quickshell.Networking
import QtQuick

Piece {
    readonly property NetworkDevice online: Networking.devices.values.find(device => device.connected) ?? null

    active: !online
    role: active ? "warning" : "text"
    value: {
        if (!online)
            return "offline";
        if (online.type === DeviceType.Wired)
            return "wired";
        const network = online.networks.values.find(network => network.connected);
        return network ? network.name : online.name;
    }
}
