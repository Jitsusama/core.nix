// The cases of cameras for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Cameras.js" as Cameras

function run(failures) {
    let asked = 0;
    const machine = Common.checker(failures, () => asked++);
    // A camera's drawer says its parts, what moves first and what
    // identifies it last, its holders left to its row's side, how long
    // its sensor's been on to the second and its modes parted by dots.
    const cameraWords = { builtIn: "built in \u00b7 camera", plugged: "on USB \u00b7 camera",
        powered: "{state} \u00b7 {lasted} since start" };
    const cameraParts = [
        { key: "camera:camera:holders", name: "In Use", side: "Zoom" },
        { key: "camera:camera:privacy", name: "Privacy Light", side: "on" },
        { key: "camera:camera:sensor", name: "Sensor", side: "OmniVision OV08X40" },
        { key: "camera:camera:where", name: "Where", side: "front, centre" },
        { key: "camera:camera:powered", name: "Powered", side: "on", active: 192400 },
        { key: "camera:camera:modes", name: "Formats", side: "MJPEG, YUYV",
            listed: ["MJPEG", "YUYV"] },
        { key: "fact:cameras:camera:usb", name: "Connection", side: "5 Gbit/s" },
        { key: "fact:cameras:camera:field of view", name: "Field of View", side: "90\u00b0" },
        { key: "camera:camera:streaming", name: "Streaming",
            side: "NV12 1920 \u00d7 1080 at 30" },
        { key: "camera:camera:flood", name: "IR Light", side: "" }
    ];
    const cameraDrawn = Cameras.drawer(cameraParts,
        { name: "Built-In Camera", builtIn: true, said: cameraWords });
    machine("a camera's drawer, in its order", cameraDrawn.facts.map(fact => fact.label),
        ["Streaming", "Privacy Light", "Powered", "Formats", "Sensor", "Where",
            "Field of View", "Connection"]);
    machine("a camera's drawer, as it says each", [cameraDrawn.eyebrow,
        cameraDrawn.facts[2].value, cameraDrawn.facts[3].value],
        ["built in \u00b7 camera", "on \u00b7 3 min 12 s since start", "MJPEG \u00b7 YUYV"]);
    machine("a plugged-in camera's eyebrow", Cameras.drawer([],
        { name: "Logitech BRIO", builtIn: false, said: cameraWords }).eyebrow,
        "on USB \u00b7 camera");
    machine("powered for hours, to its two largest units", Cameras.drawer([
        { key: "camera:camera:powered", name: "Powered", side: "on", active: 7500000 }],
        { builtIn: true, said: cameraWords }).facts[0].value,
        "on \u00b7 2 h 5 min since start");
    return asked;
}
