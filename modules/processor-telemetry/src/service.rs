//! io.jitsusama.ProcessorTelemetry, which the shell and varlinkctl ask over
//! varlink on the socket systemd holds for the reader.
//!
//! ```sh
//! varlinkctl call --more /run/processor-telemetry/io.jitsusama.ProcessorTelemetry \
//!   io.jitsusama.ProcessorTelemetry.Watch '{}'
//! ```

use std::env;
use std::fs;
use std::io;
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::process;
use std::time::Duration;

use futures_util::{Stream, stream};
use serde::{Deserialize, Serialize};
use tokio::time::{self, Instant};
use zlink::introspect::CustomType;
use zlink::tokio::unix::Listener;
use zlink::{Reply, introspect, service};

use crate::layout::{self, Kind, Limit, Sensor};
use crate::reading::{self, Cause, Held, Region};

/// How long apart the two readings each answer compares are.
pub const EVERY: Duration = Duration::from_secs(1);

pub struct Telemetry {
    /// The region's `telem` file, if the machine has the region.
    pub region: Option<PathBuf>,
    /// Each core cluster's CPUs, in the region's order.
    pub clusters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, zlink::ReplyError, introspect::ReplyError)]
#[zlink(interface = "io.jitsusama.ProcessorTelemetry")]
pub enum Unread {
    /// The processor has no telemetry region this reader knows the layout of.
    NoRegion,
    /// The region couldn't be read, for this reason.
    Unreadable { reason: String },
}

/// A part with a temperature no hwmon chip shows.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Part {
    Npu,
    ImageProcessor,
    Media,
    Display,
    SystemAgent,
    Chipset,
}

/// What held something back: a package limit, or a kind of limit.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Why {
    Pl1,
    Pl2,
    Pl3,
    PlatformPl1,
    PlatformPl2,
    PlatformPl3,
    Prochot,
    RegulatorHot,
    Heat,
    Power,
    Current,
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
pub struct Temperature {
    pub part: Part,
    pub celsius: i64,
}

/// What held something back, and for what share of the last second.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
pub struct Hold {
    pub by: Why,
    pub share: f64,
}

/// A core cluster's CPUs and what held them back.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Cluster {
    pub cpus: String,
    pub held_back: Vec<Hold>,
}

/// The temperatures now, and what held the processor, each core cluster
/// and the NPU back over the last second, each list the longest-holding
/// first and empty when nothing did.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Reading {
    pub temperatures: Vec<Temperature>,
    pub held_back: Vec<Hold>,
    pub clusters: Vec<Cluster>,
    pub npu: Vec<Hold>,
}

fn part(sensor: Sensor) -> Part {
    match sensor {
        Sensor::Npu => Part::Npu,
        Sensor::ImageProcessor => Part::ImageProcessor,
        Sensor::Media => Part::Media,
        Sensor::Display => Part::Display,
        Sensor::SystemAgent => Part::SystemAgent,
        Sensor::Chipset => Part::Chipset,
    }
}

fn kind(kind: Kind) -> Why {
    match kind {
        Kind::Heat => Why::Heat,
        Kind::Power => Why::Power,
        Kind::Current => Why::Current,
        Kind::Other => Why::Other,
    }
}

fn limit(limit: Limit) -> Why {
    match limit {
        Limit::Pl1 => Why::Pl1,
        Limit::Pl2 => Why::Pl2,
        Limit::Pl3 => Why::Pl3,
        Limit::PlatformPl1 => Why::PlatformPl1,
        Limit::PlatformPl2 => Why::PlatformPl2,
        Limit::PlatformPl3 => Why::PlatformPl3,
        Limit::Prochot => Why::Prochot,
        Limit::RegulatorHot => Why::RegulatorHot,
    }
}

fn holds<T: Copy>(held: &[Held<T>], why: impl Fn(T) -> Why) -> Vec<Hold> {
    held.iter()
        .map(|held| Hold {
            by: why(held.by),
            share: held.share,
        })
        .collect()
}

/// The reading two regions `seconds` apart make, each cluster named by
/// its CPUs where the topology gives as many clusters as the region counts.
pub fn reading(before: &Region, after: &Region, seconds: f64, cpus: &[String]) -> Reading {
    let held = reading::held_back(before, after, seconds);
    let temperatures = after
        .temperatures()
        .into_iter()
        .map(|(sensor, celsius)| Temperature {
            part: part(sensor),
            celsius: celsius.into(),
        })
        .collect();
    let named = cpus.len() == layout::CLUSTERS;
    let clusters = held
        .clusters
        .iter()
        .enumerate()
        .map(|(index, kinds)| Cluster {
            cpus: if named {
                cpus[index].clone()
            } else {
                String::new()
            },
            held_back: holds(kinds, kind),
        })
        .collect();
    Reading {
        temperatures,
        held_back: holds(&held.processor, |cause| match cause {
            Cause::Limit(l) => limit(l),
            Cause::Kind(k) => kind(k),
        }),
        clusters,
        npu: holds(&held.npu, kind),
    }
}

/// The region's `telem` file among Intel PMT's, by its guid, from `class`
/// (the kernel's `/sys/class/intel_pmt`).
pub fn find(class: &Path) -> Option<PathBuf> {
    let wanted = format!("{:#x}", layout::GUID);
    fs::read_dir(class)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find_map(|dir| {
            let guid = fs::read_to_string(dir.join("guid")).ok()?;
            (guid.trim() == wanted).then(|| dir.join("telem"))
        })
}

fn read(file: &Path) -> Result<Region, Unread> {
    let bytes = fs::read(file).map_err(|error| Unread::Unreadable {
        reason: error.to_string(),
    })?;
    Region::from_bytes(&bytes).ok_or_else(|| Unread::Unreadable {
        reason: format!(
            "{} bytes, not the layout's {}",
            bytes.len(),
            layout::WORDS * 8
        ),
    })
}

#[service(
    interface = "io.jitsusama.ProcessorTelemetry",
    types = [Cluster, Hold, Part, Reading, Temperature, Why],
    vendor = "jitsusama",
    product = "processor-telemetry",
    version = env!("CARGO_PKG_VERSION"),
    url = "https://github.com/Jitsusama/core.nix"
)]
impl Telemetry {
    /// The temperatures, and what held everything back over the next
    /// second. Called with more, it answers again each second until the
    /// call ends, reading the region only while someone is asking.
    #[zlink(more)]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "zlink's service macro awaits every method it answers with, so each must be async"
    )]
    async fn watch(
        &self,
        more: bool,
    ) -> impl Stream<Item = Result<Reply<Reading>, Unread>> + Unpin {
        let file = self.region.clone();
        let cpus = self.clusters.clone();
        let first = file.as_deref().map(read);
        let replies = stream::unfold(
            (first, Instant::now(), false),
            move |(last, at, done): (Option<Result<Region, Unread>>, Instant, bool)| {
                let file = file.clone();
                let cpus = cpus.clone();
                async move {
                    if done {
                        return None;
                    }
                    let before = match last {
                        None => return Some((Err(Unread::NoRegion), (None, at, true))),
                        Some(Err(unread)) => return Some((Err(unread), (None, at, true))),
                        Some(Ok(region)) => region,
                    };
                    time::sleep_until(at + EVERY).await;
                    let now = Instant::now();
                    let after = match file.as_deref().map(read) {
                        Some(Ok(region)) => region,
                        Some(Err(unread)) => return Some((Err(unread), (None, now, true))),
                        None => return Some((Err(Unread::NoRegion), (None, now, true))),
                    };
                    let seconds = (now - at).as_secs_f64();
                    let reading = reading(&before, &after, seconds, &cpus);
                    let reply = Reply::new(Some(reading)).set_continues(Some(more));
                    Some((Ok(reply), (Some(Ok(after)), now, !more)))
                }
            },
        );
        Box::pin(replies)
    }
}

/// The socket systemd holds for the reader, if it passed one, as the first
/// of the files it hands a service.
pub fn listener() -> io::Result<Option<Listener>> {
    let ours = env::var("LISTEN_PID").ok().and_then(|pid| pid.parse().ok()) == Some(process::id());
    if !ours || env::var("LISTEN_FDS").as_deref() != Ok("1") {
        return Ok(None);
    }
    // SAFETY: systemd handed this process the socket as file 3, its first,
    // and nothing else in the reader takes it.
    let socket = unsafe { OwnedFd::from_raw_fd(3) };
    let listener =
        Listener::try_from(socket).map_err(|error| io::Error::other(error.to_string()))?;
    Ok(Some(listener))
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;

    use super::*;

    const SECOND: u64 = 38_400_000;

    fn region_bytes(pl1: u64, npu: u8) -> Vec<u8> {
        let mut words = vec![0_u64; layout::WORDS];
        words[0] = 2 << 32;
        words[15] = u64::from(npu) << 32;
        words[150] = pl1;
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    fn sysfs(guid: &str) -> tempfile::TempDir {
        let dir =
            tempfile::tempdir().unwrap_or_else(|error| panic!("no temporary folder: {error}"));
        for (name, id) in [("telem1", "0x3086100"), ("telem2", guid)] {
            let region = dir.path().join(name);
            fs::create_dir(&region).unwrap_or_else(|error| panic!("{error}"));
            fs::write(region.join("guid"), format!("{id}\n"))
                .unwrap_or_else(|error| panic!("{error}"));
            fs::write(region.join("telem"), region_bytes(0, 40))
                .unwrap_or_else(|error| panic!("{error}"));
        }
        dir
    }

    #[test]
    fn the_region_is_found_by_its_guid() {
        let dir = sysfs("0x3086000");
        assert_eq!(find(dir.path()), Some(dir.path().join("telem2/telem")));
    }

    #[test]
    fn another_processors_regions_arent_taken() {
        let dir = sysfs("0x1234");
        assert_eq!(find(dir.path()), None);
    }

    #[test]
    fn a_reading_names_each_cluster_by_its_cpus() {
        let before = Region::from_bytes(&region_bytes(0, 40));
        let after = Region::from_bytes(&region_bytes(SECOND, 47));
        let (Some(before), Some(after)) = (before, after) else {
            panic!("the stand-in regions are the layout's size");
        };
        let cpus: Vec<String> = ["0", "1", "2", "3", "4-7", "8-11", "12-15"]
            .map(String::from)
            .to_vec();
        let reading = reading(&before, &after, 1.0, &cpus);
        assert_eq!(
            reading.held_back,
            [Hold {
                by: Why::Pl1,
                share: 1.0
            }]
        );
        assert_eq!(reading.clusters[4].cpus, "4-7");
        assert_eq!(
            reading.temperatures[0],
            Temperature {
                part: Part::Npu,
                celsius: 47
            }
        );
    }

    #[test]
    fn clusters_the_topology_doesnt_match_go_unnamed() {
        let region = Region::from_bytes(&region_bytes(0, 40));
        let Some(region) = region else {
            panic!("the stand-in region is the layout's size");
        };
        let reading = reading(&region, &region, 1.0, &["0-15".into()]);
        assert!(
            reading
                .clusters
                .iter()
                .all(|cluster| cluster.cpus.is_empty())
        );
    }

    #[tokio::test(start_paused = true)]
    async fn asked_once_it_answers_once_after_a_second() {
        let dir = sysfs("0x3086000");
        let telemetry = Telemetry {
            region: find(dir.path()),
            clusters: Vec::new(),
        };
        let mut replies = telemetry.watch(false).await;
        let first = replies.next().await;
        assert!(matches!(first, Some(Ok(_))));
        assert!(replies.next().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn without_the_region_it_says_so() {
        let telemetry = Telemetry {
            region: None,
            clusters: Vec::new(),
        };
        let mut replies = telemetry.watch(true).await;
        assert!(matches!(replies.next().await, Some(Err(Unread::NoRegion))));
        assert!(replies.next().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn watched_it_answers_each_second_with_what_changed() {
        let dir = sysfs("0x3086000");
        let file = dir.path().join("telem2/telem");
        let telemetry = Telemetry {
            region: Some(file.clone()),
            clusters: Vec::new(),
        };
        let mut replies = telemetry.watch(true).await;
        assert!(matches!(replies.next().await, Some(Ok(_))));
        fs::write(&file, region_bytes(SECOND / 2, 52)).unwrap_or_else(|error| panic!("{error}"));
        let Some(Ok(reply)) = replies.next().await else {
            panic!("no second reading");
        };
        let reading = reply.into_parameters();
        let held = reading.map(|r| r.held_back);
        assert_eq!(
            held,
            Some(vec![Hold {
                by: Why::Pl1,
                share: 0.5
            }])
        );
    }
}
