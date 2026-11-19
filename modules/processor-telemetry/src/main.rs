//! processor-telemetry: reads the processor's own telemetry, which Intel's
//! power unit keeps and the kernel shows only to root, and answers the shell
//! with what it says that nothing else does: the temperatures of the NPU,
//! the image processor, the media and display engines, the system agent and
//! the chipset, what held the processor, each core cluster and the NPU
//! back, by which limit, and what kept the package out of its deep idle
//! states, by which part, over the last second.
//!
//! systemd starts it when the shell first connects to its socket. It reads
//! the regions only while someone is watching, once a second, and otherwise
//! waits, costing nothing. Their layouts are Panther Lake's (src/layout.rs
//! and src/resting.rs); on another processor it answers that it has no
//! region it knows.
//!
//! Its unit alone starts it, in the terminal kit's shape, as
//! `processor-telemetry --help` says (`help()`).

mod clusters;
mod layout;
mod reading;
mod resting;
mod service;

use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use terminal_kit::{Exit, Failure, Help};
use zlink::Server;

/// What `processor-telemetry --help` says: how its unit runs it, and how
/// it ends.
fn help() -> Help {
    Help::new(
        "processor-telemetry",
        "answers the shell with the processor's own telemetry",
    )
    .example(
        "processor-telemetry",
        "as its unit runs it, on the socket systemd hands it",
    )
    .exit(Exit::Failed, "its socket couldn't be taken or answered on")
    .exit(Exit::Config, "systemd handed it no socket")
}

/// It runs as long as its socket's unit does, so it only ends by failing.
fn main() -> ExitCode {
    // Its ready line counts from here, so building the runtime is in it.
    let started = Instant::now();
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        if !context.args.is_empty() {
            return Err(
                Failure::new(Exit::Usage, "processor-telemetry takes no words")
                    .because("systemd hands it its socket")
                    .try_this("processor-telemetry --help"),
            );
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                Failure::new(Exit::Failed, "it couldn't start").because(error.to_string())
            })?;
        runtime.block_on(serve(started))
    })
}

async fn serve(started: Instant) -> Result<(), Failure> {
    let class = Path::new("/sys/class/intel_pmt");
    let region = service::find(class, layout::GUID);
    let resting = service::find(class, resting::GUID);
    let clusters = clusters::read(Path::new("/sys/devices/system/cpu"));
    match &region {
        Some(file) => println!(
            "<6>reads {} for {} core clusters",
            file.display(),
            clusters.len()
        ),
        None => println!("<4>no telemetry region this reader knows the layout of"),
    }
    match &resting {
        Some(file) => println!(
            "<6>reads {} for what keeps the package from resting",
            file.display()
        ),
        None => println!("<5>no region says what keeps the package from resting"),
    }
    let listener = service::listener()
        .map_err(|error| {
            Failure::new(Exit::Failed, "it can't take the socket").because(error.to_string())
        })?
        .ok_or_else(|| {
            Failure::new(Exit::Config, "systemd handed it no socket")
                .because("so nobody can ask anything")
                .try_this("systemctl start processor-telemetry.socket")
        })?;
    let telemetry = service::Telemetry {
        region,
        resting,
        clusters,
    };
    // Health times a restart from this line to the stop before it, and
    // names a daemon that says none to its owner (modules/health,
    // src/launches.rs).
    println!(
        "<6>processor-telemetry: ready in {} ms",
        started.elapsed().as_millis()
    );
    Server::new(listener, telemetry)
        .run()
        .await
        .map_err(|error| {
            Failure::new(Exit::Failed, "it stopped answering on the socket")
                .because(error.to_string())
        })
}
