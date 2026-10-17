# One window's recording for the shell's recorder, built from this
# directory, its tests run as it builds. It runs gst-launch with the plugins
# its pipeline names: PipeWire's source, VA-API's and GL's elements from the
# base and bad sets, the MP4 muxer from the good, and x264 from the ugly.
{
  lib,
  rustPlatform,
  makeWrapper,
  gst_all_1,
  pipewire,
}:
let
  plugins = [
    gst_all_1.gstreamer
    gst_all_1.gst-plugins-base
    gst_all_1.gst-plugins-good
    gst_all_1.gst-plugins-bad
    gst_all_1.gst-plugins-ugly
    pipewire
  ];
in
rustPlatform.buildRustPackage {
  pname = "record-window";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];
  postInstall = ''
    wrapProgram $out/bin/record-window \
      --prefix PATH : ${lib.makeBinPath [ gst_all_1.gstreamer ]} \
      --set GST_PLUGIN_SYSTEM_PATH_1_0 ${lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" plugins}
  '';
  meta = {
    description = "Records one window through niri's screencast";
    license = lib.licenses.mit;
    mainProgram = "record-window";
    platforms = lib.platforms.linux;
  };
}
