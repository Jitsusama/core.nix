//! Which CPUs make each core cluster the region counts for. The kernel's
//! topology gives each CPU the id of the cluster sharing its second-level
//! cache: on Panther Lake H, each performance core alone, each module of
//! four efficient cores, and the low-power module. The region numbers its
//! clusters `CCP_0` to `CCP_6`, taken here to be those in the order of their
//! ids, performance cores first, as both number them from the first core.

use std::fs;
use std::path::Path;

/// Each cluster's CPUs as Linux lists them ("0", "4-7"), in the order of
/// the clusters' ids, read from `cpu` (the kernel's `/sys/devices/system/cpu`).
pub fn read(cpu: &Path) -> Vec<String> {
    let mut pairs = Vec::new();
    let Ok(entries) = fs::read_dir(cpu) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(number) = name.to_str().and_then(|n| n.strip_prefix("cpu")) else {
            continue;
        };
        let Ok(number) = number.parse::<u32>() else {
            continue;
        };
        let cluster = fs::read_to_string(entry.path().join("topology/cluster_id"))
            .ok()
            .and_then(|id| id.trim().parse::<u32>().ok());
        if let Some(cluster) = cluster {
            pairs.push((number, cluster));
        }
    }
    group(&pairs)
}

/// Each cluster's CPUs, from each CPU's number and cluster id.
pub fn group(pairs: &[(u32, u32)]) -> Vec<String> {
    let mut ids: Vec<u32> = pairs.iter().map(|&(_, cluster)| cluster).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.iter()
        .map(|&id| {
            let mut cpus: Vec<u32> = pairs
                .iter()
                .filter(|&&(_, c)| c == id)
                .map(|&(cpu, _)| cpu)
                .collect();
            cpus.sort_unstable();
            list(&cpus)
        })
        .collect()
}

/// CPUs as Linux lists them, a run of them as its first and last.
fn list(cpus: &[u32]) -> String {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for &cpu in cpus {
        match runs.last_mut() {
            Some((_, last)) if *last + 1 == cpu => *last = cpu,
            _ => runs.push((cpu, cpu)),
        }
    }
    runs.iter()
        .map(|&(first, last)| {
            if first == last {
                first.to_string()
            } else {
                format!("{first}-{last}")
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panther_lake_hs_cores_make_seven_clusters() {
        // As the XPS 14's cores read on 2026-10-29.
        let pairs = [
            (0, 0),
            (1, 8),
            (2, 16),
            (3, 24),
            (4, 32),
            (5, 32),
            (6, 32),
            (7, 32),
            (8, 40),
            (9, 40),
            (10, 40),
            (11, 40),
            (12, 64),
            (13, 64),
            (14, 64),
            (15, 64),
        ];
        assert_eq!(group(&pairs), ["0", "1", "2", "3", "4-7", "8-11", "12-15"]);
    }

    #[test]
    fn a_broken_run_is_listed_in_pieces() {
        assert_eq!(group(&[(0, 1), (2, 1), (3, 1)]), ["0,2-3"]);
    }

    #[test]
    fn clusters_follow_their_ids_not_their_cpus() {
        assert_eq!(group(&[(0, 9), (1, 2)]), ["1", "0"]);
    }

    #[test]
    fn a_missing_topology_gives_no_clusters() {
        assert!(read(Path::new("/nonexistent")).is_empty());
    }
}
