# The launcher's maths (maths.cpp): a QML module built with the Qt
# Quickshell is built with, so the shell loads it in its own process, and
# linked to a libqalculate with nothing that reaches past the machine. Its
# exchange rates would be fetched with curl and its plots drawn by gnuplot,
# so it's built without curl and without gnuplot's path, which also keeps
# gnuplot's 200 MiB out of the shell's closure.
{
  lib,
  stdenv,
  cmake,
  ninja,
  pkg-config,
  qt6,
  libqalculate,
}:
let
  qalculate = (libqalculate.override { gnuplotBinary = ""; }).overrideAttrs (old: {
    buildInputs = lib.filter (input: (input.pname or "") != "curl") old.buildInputs;
    configureFlags = (old.configureFlags or [ ]) ++ [ "--without-libcurl" ];
  });
in
stdenv.mkDerivation {
  pname = "quickshell-maths";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./CMakeLists.txt
      ./maths.h
      ./maths.cpp
    ];
  };
  nativeBuildInputs = [
    cmake
    ninja
    pkg-config
  ];
  buildInputs = [
    qt6.qtbase
    qt6.qtdeclarative
    qalculate
  ];
  # A library, not a program, so there's nothing for Qt to wrap.
  dontWrapQtApps = true;
  cmakeFlags = [ (lib.cmakeFeature "QML_INSTALL_DIR" qt6.qtbase.qtQmlPrefix) ];
  meta = {
    description = "libqalculate as a QML module, for the launcher's sums and measures";
    license = lib.licenses.gpl2Plus;
    platforms = lib.platforms.linux;
  };
}
