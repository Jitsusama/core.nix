# The relay that plays a camera into its loopback, for every module that
# shares one (the built-in camera's, the MX Brio's), patched to start the
# camera again when a client waits with no frame, which a slow setup under
# pressure left it doing until it was restarted by hand.
{ v4l2-relayd }:
v4l2-relayd.overrideAttrs (old: {
  patches = old.patches or [ ] ++ [ ./v4l2-relayd-restarts-a-stalled-camera.patch ];
})
