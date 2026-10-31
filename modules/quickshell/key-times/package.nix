# When keys were pressed (keytimes.cpp): a QML module built with the Qt
# Quickshell is built with, so the shell loads it in its own process and
# its launcher can time a key from the moment it was pressed to its frame.
{
  lib,
  stdenv,
  cmake,
  ninja,
  qt6,
}:
stdenv.mkDerivation {
  pname = "quickshell-key-times";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./CMakeLists.txt
      ./keytimes.h
      ./keytimes.cpp
    ];
  };
  nativeBuildInputs = [
    cmake
    ninja
  ];
  buildInputs = [
    qt6.qtbase
    qt6.qtdeclarative
  ];
  # A library, not a program, so there's nothing for Qt to wrap.
  dontWrapQtApps = true;
  cmakeFlags = [ (lib.cmakeFeature "QML_INSTALL_DIR" qt6.qtbase.qtQmlPrefix) ];
  meta = {
    description = "When each key was pressed, as a QML module, for the launcher's key timing";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
