# How much of the shell's JavaScript heap is live (heap.cpp): a QML module
# built with the Qt Quickshell is built with, since it reads that Qt's
# private memory manager, so the launcher can say it beside the collection
# it runs on closing.
{
  lib,
  stdenv,
  cmake,
  ninja,
  qt6,
}:
stdenv.mkDerivation {
  pname = "quickshell-heap";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./CMakeLists.txt
      ./heap.h
      ./heap.cpp
      ./heap_test.cpp
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
  # Its test keeps and lets go of objects in a JavaScript engine.
  doCheck = true;
  cmakeFlags = [
    (lib.cmakeFeature "QML_INSTALL_DIR" qt6.qtbase.qtQmlPrefix)
    # Qt warns that private headers tie a module to the Qt it was built
    # with. That's the Qt it's given here, and any change to it builds the
    # module again, so the warning says nothing.
    (lib.cmakeBool "QT_NO_PRIVATE_MODULE_WARNING" true)
  ];
  meta = {
    description = "How much of the shell's JavaScript heap is live, as a QML module";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
