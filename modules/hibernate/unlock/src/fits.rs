//! Whether a hibernation image would fit, asked before hibernating so one
//! that can't is refused before anything changes. Once a hibernation
//! starts, the screen locks, and a rollback leaves it locked over a laptop
//! that never went off, which looks like being signed out.
//!
//! The sums are 7.3's kernel/power/snapshot.c's. The image may hold at most
//! half of memory, less what the kernel keeps back for itself, less twice
//! the reserve (`/sys/power/reserved_size`), counted in pages:
//! `hibernate_preallocate_memory`'s `max_size`, short only the few pages of
//! the image's own bitmaps. What it can't leave out is what's in use less
//! what reclaim can take back: the page cache, the kernel's reclaimable
//! slabs and the anonymous pages, which `minimum_image_size` counts, as far
//! as swap has room for those. Free pages waiting on each processor's own
//! list count as in use there, since only the free lists mark a page free,
//! but they're the first the image's own allocation takes, so here they're
//! free: on a 1 GiB machine they were a ninth of memory.
//! The floors count as nothing, since the hook sets them aside first
//! (floors.rs says why).

/// A page, as the kernel counts memory for the image.
const PAGE: u64 = 4096;

/// The pages the kernel keeps for the image's own I/O, `PAGES_FOR_IO`.
const PAGES_FOR_IO: u64 = 1024;

/// What `/proc/meminfo` says, in bytes.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Memory {
    total: u64,
    free: u64,
    file: u64,
    slab: u64,
    anon: u64,
}

impl Memory {
    pub fn parse(meminfo: &str) -> Option<Self> {
        let bytes = |name: &str| {
            meminfo.lines().find_map(|line| {
                let kib = line.strip_prefix(name)?.strip_prefix(':')?;
                kib.trim().strip_suffix("kB")?.trim().parse::<u64>().ok()
            })
        };
        let kib = |name: &str| bytes(name).map(|kib| kib * 1024);
        Some(Self {
            total: kib("MemTotal")?,
            free: kib("MemFree")?,
            file: kib("Active(file)")? + kib("Inactive(file)")?,
            slab: kib("SReclaimable")?,
            anon: kib("Active(anon)")? + kib("Inactive(anon)")?,
        })
    }
}

/// What `/proc/zoneinfo` says, in bytes.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Zones {
    /// What the kernel keeps back from every allocation,
    /// `totalreserve_pages`, as `mm/page_alloc.c`'s
    /// `calculate_totalreserve_pages` sums it: each zone's largest lowmem
    /// reserve (`protection`) and its high watermark, at most the pages it
    /// manages.
    pub kept_back: u64,
    /// The free pages waiting on each processor's own list, its pagesets'
    /// `count`, which `MemFree` leaves out.
    pub parked: u64,
}

impl Zones {
    pub fn parse(zoneinfo: &str) -> Self {
        let mut zones: Vec<(u64, u64, u64)> = Vec::new();
        let mut parked = 0;
        for line in zoneinfo.lines() {
            let line = line.trim();
            let number = |name: &str| line.strip_prefix(name)?.trim().parse::<u64>().ok();
            if line.starts_with("Node ") && line.contains(", zone ") {
                zones.push((0, 0, 0));
            } else if let Some(count) = number("count:") {
                parked += count;
            } else if let Some(zone) = zones.last_mut() {
                if let Some(high) = number("high ") {
                    zone.0 = high;
                } else if let Some(managed) = number("managed ") {
                    zone.1 = managed;
                } else if let Some(listed) = line.strip_prefix("protection:") {
                    zone.2 = listed
                        .trim_matches(|c: char| c == '(' || c == ')' || c.is_whitespace())
                        .split(',')
                        .filter_map(|reserve| reserve.trim().parse().ok())
                        .max()
                        .unwrap_or(0);
                }
            }
        }
        let kept_back = zones
            .iter()
            .map(|(high, managed, protection)| (high + protection).min(*managed))
            .sum::<u64>();
        Self {
            kept_back: kept_back * PAGE,
            parked: parked * PAGE,
        }
    }
}

/// The room left in every swap area `/proc/swaps` lists, in bytes.
pub fn swap_free(swaps: &str) -> u64 {
    swaps
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let size: u64 = fields.get(2)?.parse().ok()?;
            let used: u64 = fields.get(3)?.parse().ok()?;
            Some(size.saturating_sub(used) * 1024)
        })
        .sum()
}

/// What the image may hold and what it can't leave out, in bytes.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Fit {
    pub most: u64,
    pub least: u64,
}

impl Fit {
    pub fn of(memory: Memory, zones: Zones, reserve: u64, swap_free: u64) -> Self {
        let pages = memory.total.saturating_sub(zones.kept_back) / PAGE;
        let most = (pages.saturating_sub(PAGES_FOR_IO) / 2)
            .saturating_sub(2 * reserve.div_ceil(PAGE))
            * PAGE;
        let used = memory
            .total
            .saturating_sub(memory.free)
            .saturating_sub(zones.parked);
        let taken_back = memory.file + memory.slab + memory.anon.min(swap_free);
        Self {
            most,
            least: used.saturating_sub(taken_back),
        }
    }

    pub fn fits(self) -> bool {
        self.least <= self.most
    }

    /// What `hibernate-unlock fits` says, in words for whoever asked.
    pub fn said(self) -> String {
        let gb = |bytes: u64| {
            let tenths = (bytes + 50_000_000) / 100_000_000;
            format!("{}.{} GB", tenths / 10, tenths % 10)
        };
        if self.fits() {
            format!(
                "It fits: at least {} of memory has to be saved, and there's room for {}.",
                gb(self.least),
                gb(self.most)
            )
        } else {
            format!(
                "Not enough memory to hibernate: {} of memory can't be freed, and there's room \
                 for only {} once drivers have theirs. Nothing was changed. Close something you \
                 don't need, then hibernate again.",
                gb(self.least),
                gb(self.most)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An 8 GiB laptop's /proc/meminfo on 6 November, 15 minutes after a
    /// hibernation rolled back there, with only the lines the sums read.
    const LAPTOP: &str = "MemTotal:        7562128 kB\nMemFree:         3640004 kB\n\
        Active(anon):     577276 kB\nInactive(anon):   140680 kB\n\
        Active(file):     432996 kB\nInactive(file):  1444780 kB\n\
        SReclaimable:      95524 kB\n";

    /// Its /proc/swaps then.
    const SWAPS: &str = "Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n\
        /swap/swapfile                          file\t\t8388604\t\t2480\t\t-1\n\
        /dev/zram0                              partition\t7561212\t\t197976\t\t5\n";

    /// Its /proc/zoneinfo's zones then, with only the lines the sums read.
    const ZONES: &str = "Node 0, zone      DMA\n  pages free     3585\n        high     72\n\
        managed  3841\n        protection: (0, 1932, 7369, 7369, 7369)\n\
        Node 0, zone    DMA32\n  pages free     344948\n        high     9366\n\
        managed  494655\n        protection: (0, 0, 5437, 5437, 5437)\n\
        Node 0, zone   Normal\n  pages free     560305\n        high     26360\n\
        managed  1392036\n        protection: (0, 0, 0, 0, 0)\n\
        Node 0, zone  Movable\n  pages free     0\n        high     32\n\
        managed  0\n        protection: (0, 0, 0, 0, 0)\n";

    const GIB: u64 = 1024 * 1024 * 1024;

    fn laptop(reserve: u64, swap_free: u64) -> Fit {
        let memory = Memory::parse(LAPTOP).unwrap();
        Fit::of(memory, Zones::parse(ZONES), reserve, swap_free)
    }

    #[test]
    fn what_the_kernel_keeps_back_is_each_zones_reserve_and_high_mark_at_most_its_own() {
        // DMA's 7,369 and 72 held to its 3,841; DMA32's 5,437 and 9,366;
        // Normal's 26,360; Movable manages nothing.
        assert_eq!(
            Zones::parse(ZONES).kept_back,
            (3841 + 14_803 + 26_360) * PAGE
        );
    }

    #[test]
    fn every_processors_waiting_free_pages_count_and_nothing_else_does() {
        let zones = Zones::parse(
            "Node 0, zone    DMA32\n        high     16646\n        managed  241596\n\
             protection: (0, 0, 0, 0, 0)\n  pagesets\n    cpu: 0\n              count: 29279\n\
                           high:  29760\n              batch: 63\n    cpu: 1\n\
                           count: 21\n              high:  29760\n              batch: 63\n",
        );
        assert_eq!(zones.parked, (29_279 + 21) * PAGE);
        assert_eq!(zones.kept_back, 16_646 * PAGE);
    }

    #[test]
    fn pages_waiting_on_a_processors_list_are_free_for_the_image() {
        let memory = Memory::parse(LAPTOP).unwrap();
        let none = Zones::parse(ZONES);
        let parked = Zones {
            parked: 100 * PAGE,
            ..none
        };
        let without = Fit::of(memory, none, GIB, swap_free(SWAPS));
        let with = Fit::of(memory, parked, GIB, swap_free(SWAPS));
        assert_eq!(without.least - with.least, 100 * PAGE);
        assert_eq!(without.most, with.most);
    }

    #[test]
    fn meminfo_is_read_in_bytes() {
        let memory = Memory::parse(LAPTOP).unwrap();
        assert_eq!(memory.total, 7_562_128 * 1024);
        assert_eq!(memory.file, (432_996 + 1_444_780) * 1024);
        assert_eq!(memory.anon, (577_276 + 140_680) * 1024);
        assert_eq!(memory.slab, 95_524 * 1024);
    }

    #[test]
    fn a_meminfo_missing_a_line_is_not_read() {
        assert_eq!(Memory::parse("MemTotal: 1 kB\nMemFree: 1 kB\n"), None);
    }

    #[test]
    fn every_swap_areas_room_counts() {
        assert_eq!(
            swap_free(SWAPS),
            (8_388_604 - 2480 + 7_561_212 - 197_976) * 1024
        );
    }

    #[test]
    fn the_cap_is_half_of_memory_less_twice_the_reserve_as_the_kernel_sums_it() {
        // 1,890,532 pages, less 45,004 kept back and 1,024, halved, less
        // twice 262,144.
        assert_eq!(laptop(GIB, swap_free(SWAPS)).most, 397_964 * PAGE);
    }

    #[test]
    fn the_laptop_after_the_rollback_fits_once_the_floors_are_set_aside() {
        let fit = laptop(GIB, swap_free(SWAPS));
        // In use 3,922,124 KiB, less 1,877,776 of cache, 95,524 of slabs and
        // 717,956 anonymous.
        assert_eq!(fit.least, 1_230_868 * 1024);
        assert!(fit.fits(), "{fit:?}");
        assert_eq!(
            fit.said(),
            "It fits: at least 1.3 GB of memory has to be saved, and there's room for 1.6 GB."
        );
    }

    #[test]
    fn a_reserve_as_large_as_memory_leaves_no_room() {
        let fit = laptop(Memory::parse(LAPTOP).unwrap().total, swap_free(SWAPS));
        assert_eq!(fit.most, 0);
        assert!(!fit.fits());
        assert_eq!(
            fit.said(),
            "Not enough memory to hibernate: 1.3 GB of memory can't be freed, and there's room \
             for only 0.0 GB once drivers have theirs. Nothing was changed. Close something you \
             don't need, then hibernate again."
        );
    }

    #[test]
    fn anonymous_memory_swap_has_no_room_for_stays() {
        let with_swap = laptop(GIB, swap_free(SWAPS));
        let without = laptop(GIB, 0);
        assert_eq!(
            without.least - with_swap.least,
            Memory::parse(LAPTOP).unwrap().anon
        );
    }
}
