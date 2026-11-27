# The test files holding bounds at idle on how long a step took before
# every-timed-step-allows-for-load, each with how many, waiting while each
# comes to allow for how busy the host is. The check fails on a file holding
# more, and on one holding fewer until its count here is lowered, so the list
# only shrinks.
{
  "modules/calls/tests/a-call-is-on.rs" = 1;
  "modules/daemon-serve/src/serve/tests/hang_ups.rs" = 1;
  "modules/elsewhere/src/far/tests/crossing.rs" = 2;
  "modules/last-seconds/tests/each_sleep_is_heard.rs" = 1;
  "modules/local-sites/tests/serve.rs" = 1;
  "modules/machine/tests/daemon.rs" = 1;
  "modules/night-shift/tests/away/mod.rs" = 3;
  "modules/night-shift/tests/run.rs" = 2;
  "modules/night-shift/tests/turn.rs" = 3;
  "modules/pain-points/src/hear/tests/sends.rs" = 1;
  "modules/passwords/tests/asks/mod.rs" = 1;
  "modules/passwords/tests/common/mod.rs" = 1;
  "modules/passwords/tests/copies/mod.rs" = 1;
  "modules/passwords/tests/daemon.rs" = 1;
  "modules/passwords/tests/keys.rs" = 1;
  "modules/passwords/tests/types/mod.rs" = 1;
  "modules/people/tests/daemon.rs" = 2;
  "modules/pi/agent-proxy/tests/agent_proxy.rs" = 7;
  "modules/pi/github-door/tests/door.rs" = 1;
  "modules/pineapple/tests/reader/deaf.rs" = 4;
  "modules/pineapple/tests/reader/gateway/live.rs" = 11;
  "modules/pineapple/tests/reader/gateway/live/let_go.rs" = 1;
  "modules/pineapple/tests/reader/gateway.rs" = 4;
  "modules/pineapple/tests/reader/github/late.rs" = 11;
  "modules/pineapple/tests/reader/github/partial.rs" = 1;
  "modules/pineapple/tests/reader/github.rs" = 6;
  "modules/pineapple/tests/reader/github/watch.rs" = 3;
  "modules/pineapple/tests/reader/guard.rs" = 2;
  "modules/pineapple/tests/reader/keyring.rs" = 2;
  "modules/pineapple/tests/reader/late.rs" = 1;
  "modules/pineapple/tests/reader/live.rs" = 2;
  "modules/pineapple/tests/reader/main.rs" = 3;
  "modules/pineapple/tests/reader/said.rs" = 3;
  "modules/quickshell/tests/agents-on-niri-test.py" = 3;
  "modules/quickshell/tests/launcher_on_niri.py" = 3;
  "modules/quickshell/tests/margin-cat-on-niri-test.py" = 1;
  "modules/quickshell/tests/margin-cat-plays-on-niri-test.py" = 2;
  "modules/quickshell/tests/opening-on-niri-test.py" = 1;
  "modules/quickshell/timers/tests/daemon.rs" = 1;
  "modules/repos/tests/serve.rs" = 3;
  "modules/slack/sign-in-program/tests/printed.rs" = 1;
  "modules/update-to-main/tests/update/main.rs" = 1;
  "tests/desktop/looks/desktop-looks-vi-vm-test.py" = 1;
  "tests/vm/boot/plymouth-restore-vm-test.py" = 1;
  "tests/vm/boot/plymouth-vm-test.py" = 1;
}
