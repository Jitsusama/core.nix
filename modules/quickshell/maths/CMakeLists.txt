# The launcher's maths as a QML module, Maths, holding the singleton Maths:
# a plugin, its qmldir and its type file, installed where Qt looks for QML
# modules, so Quickshell imports it and qmllint reads its types.
cmake_minimum_required(VERSION 3.21)
project(maths LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 6.5 REQUIRED COMPONENTS Qml)
find_package(PkgConfig REQUIRED)
pkg_check_modules(QALCULATE REQUIRED IMPORTED_TARGET libqalculate)

qt_standard_project_setup(REQUIRES 6.5)

# The plugin is the module's own library, so there's one file to load.
qt_add_qml_module(maths
    URI Maths
    VERSION 1.0
    PLUGIN_TARGET maths
    OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/Maths
    SOURCES maths.h maths.cpp
)

# Every warning is an error, as the repository's other code is linted.
target_compile_options(maths PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(maths PRIVATE Qt6::Qml PkgConfig::QALCULATE)

install(TARGETS maths LIBRARY DESTINATION ${QML_INSTALL_DIR}/Maths)
install(FILES
    ${CMAKE_BINARY_DIR}/Maths/qmldir
    ${CMAKE_BINARY_DIR}/Maths/maths.qmltypes
    DESTINATION ${QML_INSTALL_DIR}/Maths
)
