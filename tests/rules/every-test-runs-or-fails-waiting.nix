# The test files holding skips that pass a test that didn't run before
# every-test-runs-or-fails, each with how many, waiting while each test runs
# where it can and fails where it can't. The check fails on a file holding
# more, and on one holding fewer until its count here is lowered, so the list
# only shrinks.
{
  "modules/disk-guard/tests/guard/anchor.rs" = 3;
  "modules/disk-guard/tests/guard/daemon.rs" = 1;
  "modules/disk-guard/tests/guard/pool.rs" = 1;
  "modules/disk-guard/tests/guard/reclaim.rs" = 2;
}
