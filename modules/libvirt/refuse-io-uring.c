// Runs a program with io_uring_setup refused, as if the kernel had no
// io_uring, so QEMU's main loop falls back to epoll. The filter is QEMU's
// alone: on the daemon that starts it, it would also take away the privilege
// virtiofsd's newuidmap and qemu-bridge-helper need.
#include <errno.h>
#include <seccomp.h>
#include <stdio.h>
#include <unistd.h>

int main(int argc, char **argv) {
  if (argc < 2) {
    fprintf(stderr, "usage: %s program [argument...]\n", argv[0]);
    return 2;
  }
  scmp_filter_ctx filter = seccomp_init(SCMP_ACT_ALLOW);
  if (filter == NULL ||
      seccomp_rule_add(filter, SCMP_ACT_ERRNO(ENOSYS), SCMP_SYS(io_uring_setup), 0) != 0 ||
      seccomp_load(filter) != 0) {
    fprintf(stderr, "%s: could not refuse io_uring\n", argv[0]);
    return 127;
  }
  seccomp_release(filter);
  execv(argv[1], argv + 1);
  perror(argv[1]);
  return 127;
}
