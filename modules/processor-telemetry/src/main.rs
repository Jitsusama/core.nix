//! processor-telemetry: reads the processor's own telemetry, which Intel's
//! power unit keeps and the kernel shows only to root, and answers the shell
//! with what it says that nothing else does: the temperatures of the NPU,
//! the image processor, the media and display engines, the system agent and
//! the chipset, and what held the processor, each core cluster and the NPU
//! back, by which limit, over the last second.
//!
//! systemd starts it when the shell first connects to its socket. It reads
//! the region only while someone is watching, once a second, and otherwise
//! waits, costing nothing. Its layout is Panther Lake's (src/layout.rs); on
//! another processor it answers that it has no region it knows.

mod clusters;
mod layout;
mod reading;
mod service;

use std::path::Path;
use std::process::ExitCode;

use zlink::Server;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let region = service::find(Path::new("/sys/class/intel_pmt"));
    let clusters = clusters::read(Path::new("/sys/devices/system/cpu"));
    match &region {
        Some(file) => println!(
            "<6>reads {} for {} core clusters",
            file.display(),
            clusters.len()
        ),
        None => println!("<4>no telemetry region this reader knows the layout of"),
    }
    let listener = match service::listener() {
        Ok(Some(listener)) => listener,
        Ok(None) => {
            println!("<3>systemd passed no socket, so nobody can ask anything");
            return ExitCode::FAILURE;
        }
        Err(error) => {
            println!("<3>can't take the socket: {error}");
            return ExitCode::FAILURE;
        }
    };
    let telemetry = service::Telemetry { region, clusters };
    match Server::new(listener, telemetry).run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            println!("<3>stopped answering on the socket: {error}");
            ExitCode::FAILURE
        }
    }
}
