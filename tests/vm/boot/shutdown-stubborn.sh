# shellcheck shell=bash
# Stands in, in shutdown.nix, for a program in an account that takes SIGTERM and never
# exits, as one did on 7 November: it ignores the signal and sleeps until it's killed. The sleep
# is its first argument, a path in the store.
trap '' TERM
exec "$1" infinity
