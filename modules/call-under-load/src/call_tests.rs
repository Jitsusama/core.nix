//! The test call's own tests, kept beside it so `call.rs` reads as the
//! call's steps alone.

use super::*;

#[test]
fn a_test_chrome_takes_the_fake_camera_and_keeps_off_joel_s_secrets() {
    let memory = Path::new("/run/x/call-under-load-1");
    let command = chrome_command(
        Path::new("chrome"),
        Path::new("/p"),
        memory,
        &memory.join("config"),
    );
    let args: Vec<_> = command.get_args().filter_map(|a| a.to_str()).collect();
    for flag in [
        "--use-fake-device-for-media-stream=fps=30",
        "--use-fake-ui-for-media-stream",
        "--password-store=basic",
        "--disable-background-networking",
    ] {
        assert!(args.contains(&flag), "{flag} is missing from {args:?}");
    }
    let env: Vec<_> = command
        .get_envs()
        .filter_map(|(k, v)| Some((k.to_str()?, v?.to_str()?)))
        .collect();
    for (key, value) in [
        ("HOME", "/run/x/call-under-load-1/home"),
        ("XDG_CONFIG_HOME", "/run/x/call-under-load-1/config"),
        ("TSS2_FAPICONF", "/nonexistent/fapi-config.json"),
        ("TPM2_PKCS11_TCTI", "mssim:host=127.0.0.1,port=9"),
    ] {
        assert!(
            env.contains(&(key, value)),
            "{key}={value} is missing from {env:?}"
        );
    }
}

fn on() -> Call {
    Call::On
}

fn off() -> Call {
    Call::Off
}

#[test]
fn a_call_of_joel_s_stops_a_step_and_the_call_s_beat() {
    let watch = Watch::asking(on, Duration::from_millis(10));
    let started = Instant::now();
    let beat = watch.wait_until(started + Duration::from_secs(3));
    assert!(
        matches!(beat, Err(Ended::Stopped(ref why)) if why == "Joel's call began"),
        "the call's beat ran on through Joel's call"
    );
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "the beat waited {:?} after Joel's call began",
        started.elapsed()
    );
    // Dropped well within its grace, so the watch never ends the tests.
    drop(watch);
    let watch = Watch::asking(on, Duration::from_millis(10));
    assert!(heard(&watch), "a step went on through Joel's call");
}

/// Whether the watch hears of a call within ten seconds: its first ask
/// comes at once, but these tests run in the package's build, which can
/// share a loaded builder, so a fixed sleep could end before it.
fn heard(watch: &Watch) -> bool {
    let until = Instant::now() + Duration::from_secs(10);
    while watch.stop().is_ok() {
        if Instant::now() > until {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
    true
}

#[test]
fn with_no_call_on_the_beat_keeps_its_time() {
    let watch = Watch::asking(off, Duration::from_millis(10));
    let started = Instant::now();
    assert!(
        watch
            .wait_until(started + Duration::from_millis(200))
            .is_ok(),
        "the beat stopped with no call on"
    );
    assert!(
        started.elapsed() >= Duration::from_millis(200),
        "the beat came early, after {:?}",
        started.elapsed()
    );
    assert!(watch.stop().is_ok(), "a step stopped with no call on");
}

#[test]
fn a_step_outlasting_its_grace_has_what_the_call_started_ended() {
    let mut stuck = Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("sleep starts");
    let watch = Watch::asking(on, Duration::from_millis(10));
    watch.ends(stuck.id());
    let started = Instant::now();
    let status = stuck.wait().expect("sleep is waited on");
    assert!(
        !status.success() && started.elapsed() < GRACE + Duration::from_secs(2),
        "the stuck process ended {status} after {:?}",
        started.elapsed()
    );
    assert!(watch.stop().is_err(), "the call didn't hear why it stopped");
}

#[test]
fn a_step_broken_off_for_joel_s_call_counts_as_stopped_and_any_other_as_failed() {
    let broken = || Err(Step::Failed("Chrome's DevTools pipe closed".into()));
    let watch = Watch::asking(on, Duration::from_millis(10));
    assert!(heard(&watch), "the watch never heard of Joel's call");
    assert!(
        matches!(settled(broken(), &watch), Ok(Ended::Stopped(ref why)) if why == "Joel's call began"),
        "a step the watch broke off read as a failed hour"
    );
    drop(watch);
    let watch = Watch::asking(off, Duration::from_millis(10));
    assert!(
        matches!(settled(broken(), &watch), Err(ref why) if why.contains("pipe closed")),
        "a step that failed by itself read as stopped"
    );
}

#[test]
fn only_a_child_of_the_call_s_is_ever_ended() {
    // This program isn't its own child, so it's left be.
    cdp::end(std::process::id());
    let mut child = Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("sleep starts");
    cdp::end(child.id());
    let status = child.wait().expect("sleep is waited on");
    assert!(!status.success(), "a child of ours wasn't ended: {status}");
}

#[test]
fn a_call_that_never_connected_fails() {
    let new = [json!({"state": "new"}), json!({"state": "connecting"})];
    assert!(
        matches!(connected(&new), Err(Step::Failed(_))),
        "a call that never connected passed"
    );
    assert!(
        connected(&[json!({"state": "connected"})]).is_ok(),
        "a connected call failed"
    );
}

#[test]
fn only_the_fake_camera_is_taken_at_either_end() {
    assert!(
        fake(&json!({"label": "fake_device_0"}), "near").is_ok(),
        "Chromium's fake camera was refused"
    );
    let refused = |said: Value| matches!(fake(&said, "far"), Err(Step::Refused(_)));
    assert!(
        refused(json!({"label": "MX Brio 705 for Business"})),
        "a real camera was taken"
    );
    assert!(refused(json!({})), "a camera with no name is refused");
}

#[test]
fn a_browser_that_enrolled_ends_the_call_and_keeps_the_test_off() {
    let temp = std::env::temp_dir();
    let profile = temp.join(format!("cul-enrol-{}", std::process::id()));
    let state = temp.join(format!("cul-enrol-state-{}", std::process::id()));
    let _ = (fs::remove_dir_all(&profile), fs::remove_dir_all(&state));
    fs::create_dir_all(profile.join("Policy/Enrollment")).expect("the folder is made");
    assert!(
        unenrolled(&[profile.as_path()], &state).is_ok(),
        "an empty enrolment folder stopped the call"
    );
    fs::write(profile.join("Policy/Enrollment/token"), "t").expect("the token is written");
    let stopped = unenrolled(&[Path::new("/nonexistent"), profile.as_path()], &state);
    let off = managed::stays_off(&state);
    let _ = (fs::remove_dir_all(&profile), fs::remove_dir_all(&state));
    assert!(
        matches!(stopped, Err(Step::Refused(ref why)) if why.contains("enrolled")),
        "an enrolled browser's call went on"
    );
    assert!(
        off.is_some_and(|why| why.contains("enrolled")),
        "the next hour's test would start again"
    );
}

#[test]
fn an_enrolment_is_kept_however_the_call_ends() {
    let temp = std::env::temp_dir();
    let profile = temp.join(format!("cul-ended-{}", std::process::id()));
    let state = temp.join(format!("cul-ended-state-{}", std::process::id()));
    let _ = fs::remove_dir_all(&state);
    fs::create_dir_all(profile.join("Policy/Enrollment")).expect("the folder is made");
    fs::write(profile.join("Policy/Enrollment/token"), "t").expect("the token is written");
    drop(Started {
        folders: vec![profile.clone()],
        children: Vec::new(),
        profiles: vec![profile.clone()],
        state: state.clone(),
    });
    let off = managed::stays_off(&state);
    let _ = fs::remove_dir_all(&state);
    assert!(!profile.exists(), "the call's profile was left behind");
    assert!(
        off.is_some_and(|why| why.contains("enrolled")),
        "an enrolment found only as the call ended was lost with its profile"
    );
}

#[test]
fn an_alarm_is_never_read_as_a_skip_for_joel_s_call() {
    let watch = Watch::asking(on, Duration::from_millis(10));
    assert!(heard(&watch), "the watch never heard of Joel's call");
    let alarm = Err(Step::Refused("a test browser enrolled".into()));
    assert!(
        matches!(settled(alarm, &watch), Err(ref why) if why.contains("enrolled")),
        "an alarm during Joel's call read as a skipped hour"
    );
}

#[test]
fn a_sweep_takes_only_what_a_gone_run_left() {
    let folder = std::env::temp_dir().join(format!("cul-sweep-{}", std::process::id()));
    let mine = folder.join(format!("profile-{}", std::process::id()));
    let gone = folder.join("profile-4294967294");
    let other = folder.join("profile-notes");
    for path in [&mine, &gone, &other] {
        fs::create_dir_all(path).unwrap();
    }
    let state = folder.join("state");
    assert_eq!(
        sweep(&folder, "profile-", &state),
        Ok(()),
        "a clean sweep refused"
    );
    assert!(mine.exists(), "a running call's folder stays");
    assert!(!gone.exists(), "a gone run's folder goes");
    assert!(other.exists(), "a folder not named for a run stays");
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn a_sweep_keeps_an_enrolment_a_killed_run_left() {
    let folder = std::env::temp_dir().join(format!("cul-swept-{}", std::process::id()));
    let gone = folder.join("call-under-load-4294967294");
    let state = folder.join("state");
    fs::create_dir_all(gone.join("far/Policy/Enrollment")).unwrap();
    fs::write(gone.join("far/Policy/Enrollment/token"), "t").unwrap();
    let swept = sweep(&folder, "call-under-load-", &state);
    let off = managed::stays_off(&state);
    let left = gone.exists();
    fs::remove_dir_all(&folder).unwrap();
    assert!(
        swept.is_err_and(|why| why.contains("enrolled")),
        "a killed run's enrolment was swept away without refusing the run"
    );
    assert!(off.is_some(), "a killed run's enrolment wasn't kept");
    assert!(!left, "the killed run's folder was left to be found again");
}

#[test]
fn a_sweep_looks_in_both_of_a_killed_run_s_profiles_in_the_cache() {
    for prefix in ["profile-", "far-"] {
        let folder = std::env::temp_dir().join(format!("cul-cache-{prefix}{}", std::process::id()));
        let gone = folder.join(format!("{prefix}4294967294"));
        let state = folder.join("state");
        fs::create_dir_all(gone.join("Policy/Enrollment")).unwrap();
        fs::write(gone.join("Policy/Enrollment/token"), "t").unwrap();
        let swept = swept(&folder.join("runtime"), &folder, &state);
        let off = managed::stays_off(&state);
        fs::remove_dir_all(&folder).unwrap();
        assert!(
            swept.is_err_and(|why| why.contains("enrolled")),
            "a killed run's {prefix} profile was swept unread"
        );
        assert!(
            off.is_some(),
            "a killed run's {prefix} enrolment wasn't kept"
        );
    }
}

#[test]
fn an_enrolment_that_can_t_be_kept_stays_where_it_is() {
    let folder = std::env::temp_dir().join(format!("cul-unkept-{}", std::process::id()));
    let profile = folder.join("profile-4294967294");
    // A file where the state folder should be, so nothing can be kept.
    let state = folder.join("state");
    fs::create_dir_all(profile.join("Policy/Enrollment")).unwrap();
    fs::write(profile.join("Policy/Enrollment/token"), "t").unwrap();
    fs::write(&state, "not a folder").unwrap();
    drop(Started {
        folders: vec![profile.clone()],
        children: Vec::new(),
        profiles: vec![profile.clone()],
        state: state.clone(),
    });
    let ended_left = profile.exists();
    let swept = sweep(&folder, "profile-", &state);
    let sweep_left = profile.exists();
    fs::remove_dir_all(&folder).unwrap();
    assert!(
        ended_left,
        "an enrolment that couldn't be kept went with the call"
    );
    assert!(
        swept.is_err(),
        "the sweep let a run start past an unkept enrolment"
    );
    assert!(
        sweep_left,
        "the sweep removed an enrolment it couldn't keep"
    );
}
