# The GTK 3 module that has a program's own file dialogs ask the portal
# (dialogs-ask-the-portal.c), as one shared library, every warning an error.
{
  gtk3,
  pkg-config,
  runCommandCC,
}:
runCommandCC "dialogs-ask-the-portal"
  {
    nativeBuildInputs = [ pkg-config ];
    buildInputs = [ gtk3 ];
  }
  ''
    mkdir -p "$out/lib"
    # shellcheck disable=SC2046 # pkg-config's flags are words.
    $CC -shared -fPIC -O2 -Wall -Wextra -Werror \
      $(pkg-config --cflags gtk+-3.0) ${./dialogs-ask-the-portal.c} \
      -o "$out/lib/libdialogs-ask-the-portal.so" $(pkg-config --libs gtk+-3.0)
  ''
