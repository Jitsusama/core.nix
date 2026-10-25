# shellcheck shell=bash
# Runs the qemu given first with the arguments after it once every socket it's
# to connect to is there, and fails naming the one still missing after a
# minute. nixpkgs' VM script
# starts each virtiofsd in the background and qemu straight after, so on a busy
# host qemu can look for a virtiofsd's socket before it's made, and fail;
# the test driver then reads qemu's output for good, since the virtiofsd still
# holds it. ./qemu-after-its-sockets.nix puts this in front of each
# qemu-system program. QEMU_SOCKETS_WAIT, in seconds, shortens the wait for
# its own test.

real=$1
shift

# The sockets qemu connects to are the -chardev sockets it doesn't serve.
sockets=()
previous=
for argument in "$@"; do
  if [ "$previous" = -chardev ] && [[ $argument == socket,* ]]; then
    path=
    serves=
    IFS=, read -r -a options <<< "$argument"
    for option in "${options[@]}"; do
      case $option in
        path=*) path=${option#path=} ;;
        server | server=on) serves=yes ;;
      esac
    done
    if [ -n "$path" ] && [ -z "$serves" ]; then
      sockets+=("$path")
    fi
  fi
  previous=$argument
done

# Each socket's folder is watched for it, a second at a time, so a socket
# made between the look and the watch is still seen.
deadline=$((SECONDS + ${QEMU_SOCKETS_WAIT:-60}))
for socket in "${sockets[@]}"; do
  folder=$(dirname "$socket")
  while [ ! -S "$socket" ] && [ -d "$folder" ] && [ "$SECONDS" -lt "$deadline" ]; do
    inotifywait -qq -t 1 -e create -e moved_to "$folder" || true
  done
  if [ ! -S "$socket" ]; then
    echo "qemu-after-its-sockets: $socket never came, so qemu wasn't started." >&2
    # The virtiofsd the script started are this process's children, and
    # while one lives, the driver waits on the output it holds.
    pkill -P $$ || true
    exit 1
  fi
done

exec "$real" "$@"
