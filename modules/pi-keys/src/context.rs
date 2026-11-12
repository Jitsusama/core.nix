//! What a sample ran beside, so a rise in its waits can be told apart:
//! how contended the machine's CPU was over it, and which pi and which
//! versions of pi's packages the windows it read were running. A rise
//! with the CPU contended is likely the load; one that starts with a new
//! install is likely the install.

use std::fs;
use std::path::Path;

/// How long, in microseconds since boot, some task has waited for a CPU,
/// from /proc/pressure/cpu's `some` line.
pub fn waited(_pressure: &str) -> Option<u64> {
    None
}

/// The one-minute load average, from /proc/loadavg.
pub fn load1(_loadavg: &str) -> Option<f64> {
    None
}

/// The pi a process runs, as its package folder's name (`pi-1.5.0`), from
/// its environment's bytes; nothing else of the environment is kept.
pub fn pi_version(_environ: &[u8]) -> Option<String> {
    None
}

/// The versions of pi's packages the process `pid` says it loaded, each
/// as `<package>@<first twelve of its version>`, from the `loaded/<pid>`
/// file each package keeps under pi-packages' folder `root`.
pub fn loaded_by(_root: &Path, _pid: u32) -> Vec<String> {
    Vec::new()
}

/// The package and version a `loaded/<pid>` file's text names.
fn named(text: &str) -> Option<String> {
    let _ = (text, fs::read_dir("."));
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cpu_s_waiting_is_read_from_its_some_line() {
        let pressure = "some avg10=69.32 avg60=50.70 avg300=28.76 total=26388679130\n\
                        full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n";
        assert_eq!(waited(pressure), Some(26_388_679_130));
        assert_eq!(waited("full total=3\n"), None, "no some line");
    }

    #[test]
    fn the_load_is_its_first_figure() {
        assert_eq!(load1("40.96 27.16 20.89 56/6502 1581827\n"), Some(40.96));
        assert_eq!(load1(""), None);
    }

    #[test]
    fn only_pi_s_package_folder_is_read_from_the_environment() {
        let environ = b"PI_PROXY_API_KEY=sk-FAKE-7QEY1S\0PI_PACKAGE_DIR=/h/.pi/pkg/pi-1.5.0\0";
        assert_eq!(pi_version(environ), Some("pi-1.5.0".into()));
        assert_eq!(pi_version(b"HOME=/h\0"), None);
    }

    #[test]
    fn a_loaded_file_names_its_package_and_version() {
        let text = r#"{"root":"/h/pi-packages/github.com/someone/a-harness/versions/d0101aa7e95a21a04390fced6f3fbc5f73c05899"}"#;
        assert_eq!(named(text), Some("a-harness@d0101aa7e95a".into()));
        assert_eq!(named(r#"{"root":"/h/elsewhere"}"#), None);
    }

    #[test]
    fn each_package_a_pi_loaded_is_found_by_its_pid() {
        let root = std::env::temp_dir().join(format!("pi-keys-loaded-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (package, version) in [("one", "aaaaaaaaaaaaaaaa"), ("two", "bbbbbbbbbbbbbbbb")] {
            let loaded = root.join("github.com/someone").join(package).join("loaded");
            fs::create_dir_all(&loaded).expect("a temporary folder is made");
            let at = root
                .join("github.com/someone")
                .join(package)
                .join("versions")
                .join(version);
            fs::write(
                loaded.join("42"),
                format!(r#"{{"root":"{}"}}"#, at.display()),
            )
            .expect("a temporary file is written");
        }
        assert_eq!(
            loaded_by(&root, 42),
            vec![
                "one@aaaaaaaaaaaa".to_string(),
                "two@bbbbbbbbbbbb".to_string()
            ]
        );
        assert!(
            loaded_by(&root, 43).is_empty(),
            "another pid loaded nothing"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
