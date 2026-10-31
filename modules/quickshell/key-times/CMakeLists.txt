# When keys were pressed, as a QML module, KeyTimes, holding the singleton
# KeyTimes: a plugin, its qmldir and its type file, installed where Qt looks
# for QML modules, so Quickshell imports it and qmllint reads its types.
cmake_minimum_required(VERSION 3.21)
project(keytimes LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 6.5 REQUIRED COMPONENTS Gui Qml)

qt_standard_project_setup(REQUIRES 6.5)

# The plugin is the module's own library, so there's one file to load.
qt_add_qml_module(keytimes
    URI KeyTimes
    VERSION 1.0
    PLUGIN_TARGET keytimes
    OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/KeyTimes
    SOURCES keytimes.h keytimes.cpp
)

# Every warning is an error, as the repository's other code is linted.
target_compile_options(keytimes PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(keytimes PRIVATE Qt6::Gui Qt6::Qml)

install(TARGETS keytimes LIBRARY DESTINATION ${QML_INSTALL_DIR}/KeyTimes)
install(FILES
    ${CMAKE_BINARY_DIR}/KeyTimes/qmldir
    ${CMAKE_BINARY_DIR}/KeyTimes/keytimes.qmltypes
    DESTINATION ${QML_INSTALL_DIR}/KeyTimes
)
