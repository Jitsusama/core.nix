# How much of the shell's JavaScript heap is live, as a QML module, Heap,
# holding the singleton Heap: a plugin, its qmldir and its type file,
# installed where Qt looks for QML modules, so Quickshell imports it and
# qmllint reads its types.
cmake_minimum_required(VERSION 3.21)
project(heap LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Qt's private targets are found only when named.
find_package(Qt6 6.5 REQUIRED COMPONENTS Qml QmlPrivate)

qt_standard_project_setup(REQUIRES 6.5)

# The plugin is the module's own library, so there's one file to load.
qt_add_qml_module(heap
    URI Heap
    VERSION 1.0
    PLUGIN_TARGET heap
    OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/Heap
    SOURCES heap.h heap.cpp
)

# Every warning is an error, as the repository's other code is linted. The
# memory manager's counts are Qt's private API (heap.cpp).
target_compile_options(heap PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(heap PRIVATE Qt6::Qml Qt6::QmlPrivate)

# What it says of a heap that keeps and lets go, run as the module is built
# (heap_test.cpp), and never installed.
enable_testing()
qt_add_executable(heap_test heap_test.cpp heap.h heap.cpp)
target_compile_options(heap_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(heap_test PRIVATE Qt6::Qml Qt6::QmlPrivate)
add_test(NAME heap_test COMMAND heap_test)

install(TARGETS heap LIBRARY DESTINATION ${QML_INSTALL_DIR}/Heap)
install(FILES
    ${CMAKE_BINARY_DIR}/Heap/qmldir
    ${CMAKE_BINARY_DIR}/Heap/heap.qmltypes
    DESTINATION ${QML_INSTALL_DIR}/Heap
)
