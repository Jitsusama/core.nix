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
//! as swap has room for those. What goes to zram stays in memory, compressed,
//! so only what compression saves is freed. Free pages waiting on each processor's own
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

/// A swap area `/proc/swaps` lists: the room left in it, in bytes, its
/// priority, and whether it's zram, which keeps what it takes in memory,
/// compressed, so that goes into the image too.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Area {
    pub room: u64,
    pub priority: i64,
    pub in_memory: bool,
}

pub fn swap_areas(swaps: &str) -> Vec<Area> {
    swaps
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let size: u64 = fields.get(2)?.parse().ok()?;
            let used: u64 = fields.get(3)?.parse().ok()?;
            Some(Area {
                room: size.saturating_sub(used) * 1024,
                priority: fields.get(4)?.parse().ok()?,
                in_memory: fields.first()?.starts_with("/dev/zram"),
            })
        })
        .collect()
}

/// What zram holds and the memory it takes to hold it, in bytes, summed
/// over every device's `mm_stat` (its first and third numbers).
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Squeeze {
    pub holds: u64,
    pub takes: u64,
}

impl Squeeze {
    pub fn parse(mm_stats: &[String]) -> Self {
        mm_stats.iter().fold(Self::default(), |sum, stat| {
            let numbers: Vec<u64> = stat
                .split_whitespace()
                .filter_map(|number| number.parse().ok())
                .collect();
            match (numbers.first(), numbers.get(2)) {
                (Some(holds), Some(takes)) => Self {
                    holds: sum.holds + holds,
                    takes: sum.takes + takes,
                },
                _ => sum,
            }
        })
    }

    /// What `bytes` swapped out to zram still take of memory, at the share
    /// it takes of what it holds now, never more than they took before.
    /// Holding nothing yet, it counts on half.
    fn cost(self, bytes: u64) -> u64 {
        if self.holds == 0 {
            return bytes / 2;
        }
        let cost = u128::from(bytes) * u128::from(self.takes) / u128::from(self.holds);
        u64::try_from(cost).unwrap_or(bytes).min(bytes)
    }
}

/// What the image may hold and what it can't leave out, in bytes.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Fit {
    pub most: u64,
    pub least: u64,
}

impl Fit {
    pub fn of(memory: Memory, zones: Zones, reserve: u64, swap: &[Area], squeeze: Squeeze) -> Self {
        let pages = memory.total.saturating_sub(zones.kept_back) / PAGE;
        let most = (pages.saturating_sub(PAGES_FOR_IO) / 2)
            .saturating_sub(2 * reserve.div_ceil(PAGE))
            * PAGE;
        let used = memory
            .total
            .saturating_sub(memory.free)
            .saturating_sub(zones.parked);
        // Anonymous pages go to the swap areas in their order, the highest
        // priority first, as the kernel fills them.
        let mut areas = swap.to_vec();
        areas.sort_by_key(|area| std::cmp::Reverse(area.priority));
        let mut left = memory.anon;
        let mut freed = 0;
        for area in areas {
            let took = left.min(area.room);
            left -= took;
            freed += if area.in_memory {
                took - squeeze.cost(took)
            } else {
                took
            };
        }
        let taken_back = memory.file + memory.slab + freed;
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

    /// An 8 GiB laptop's /proc/meminfo on 6 November, an hour after a
    /// hibernation rolled back there, with only the lines the sums read.
    const LAPTOP: &str = "MemTotal:        7562128 kB\nMemFree:         3508048 kB\n\
        Active(anon):     576328 kB\nInactive(anon):   140680 kB\n\
        Active(file):     458652 kB\nInactive(file):  1547624 kB\n\
        SReclaimable:     107788 kB\n";

    /// Its /proc/swaps then.
    const SWAPS: &str = "Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n\
        /swap/swapfile                          file\t\t8388604\t\t2476\t\t-1\n\
        /dev/zram0                              partition\t7561212\t\t196016\t\t5\n";

    /// Its zram's `mm_stat` then.
    const MM_STAT: &str =
        "146587648 26195180 32579584        0 1898442752     1891  2317817        9    16659";

    /// Its /proc/zoneinfo's zones then, with only the lines the sums read,
    /// each zone's twelve processors' counts summed into one.
    const ZONES: &str = "Node 0, zone      DMA\n        high     72\n        managed  3841\n\
        protection: (0, 1932, 7369, 7369, 7369)\n              count:    0\n\
        Node 0, zone    DMA32\n        high     9366\n        managed  494655\n\
        protection: (0, 0, 5437, 5437, 5437)\n              count:    4137\n\
        Node 0, zone   Normal\n        high     26360\n        managed  1392036\n\
        protection: (0, 0, 0, 0, 0)\n              count:    11232\n\
        Node 0, zone  Movable\n        high     32\n        managed  0\n\
        protection: (0, 0, 0, 0, 0)\n";

    const GIB: u64 = 1024 * 1024 * 1024;

    fn squeeze() -> Squeeze {
        Squeeze::parse(&[MM_STAT.to_owned()])
    }

    fn laptop(reserve: u64) -> Fit {
        let memory = Memory::parse(LAPTOP).unwrap();
        Fit::of(
            memory,
            Zones::parse(ZONES),
            reserve,
            &swap_areas(SWAPS),
            squeeze(),
        )
    }

    /// The laptop's anonymous pages swapped out to a single area.
    fn into(area: Area, squeeze: Squeeze) -> Fit {
        let memory = Memory::parse(LAPTOP).unwrap();
        Fit::of(memory, Zones::parse(ZONES), GIB, &[area], squeeze)
    }

    const ZRAM: Area = Area {
        room: 8 * GIB,
        priority: 5,
        in_memory: true,
    };

    const FILE: Area = Area {
        room: 8 * GIB,
        priority: -1,
        in_memory: false,
    };

    const QUARTER: Squeeze = Squeeze { holds: 4, takes: 1 };

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
        let zones = Zones::parse(ZONES);
        assert_eq!(zones.parked, (4137 + 11_232) * PAGE);
        let more = Zones {
            parked: zones.parked + 100 * PAGE,
            ..zones
        };
        let fit = |zones| Fit::of(memory, zones, GIB, &swap_areas(SWAPS), squeeze());
        assert_eq!(fit(zones).least - fit(more).least, 100 * PAGE);
        assert_eq!(fit(zones).most, fit(more).most);
    }

    #[test]
    fn meminfo_is_read_in_bytes() {
        let memory = Memory::parse(LAPTOP).unwrap();
        assert_eq!(memory.total, 7_562_128 * 1024);
        assert_eq!(memory.file, (458_652 + 1_547_624) * 1024);
        assert_eq!(memory.anon, (576_328 + 140_680) * 1024);
        assert_eq!(memory.slab, 107_788 * 1024);
    }

    #[test]
    fn a_meminfo_missing_a_line_is_not_read() {
        assert_eq!(Memory::parse("MemTotal: 1 kB\nMemFree: 1 kB\n"), None);
    }

    #[test]
    fn every_swap_area_is_read_with_its_room_its_priority_and_whether_its_zram() {
        assert_eq!(
            swap_areas(SWAPS),
            [
                Area {
                    room: (8_388_604 - 2476) * 1024,
                    priority: -1,
                    in_memory: false,
                },
                Area {
                    room: (7_561_212 - 196_016) * 1024,
                    priority: 5,
                    in_memory: true,
                },
            ]
        );
    }

    #[test]
    fn zram_is_read_as_what_it_holds_and_the_memory_that_takes() {
        assert_eq!(
            squeeze(),
            Squeeze {
                holds: 146_587_648,
                takes: 32_579_584,
            }
        );
    }

    #[test]
    fn anonymous_memory_zram_takes_is_freed_only_by_what_compression_saves() {
        let anon = Memory::parse(LAPTOP).unwrap().anon;
        assert_eq!(
            into(ZRAM, QUARTER).least - into(FILE, QUARTER).least,
            anon / 4
        );
    }

    #[test]
    fn an_empty_zram_counts_on_half() {
        let anon = Memory::parse(LAPTOP).unwrap().anon;
        assert_eq!(
            into(ZRAM, Squeeze::default()).least - into(FILE, QUARTER).least,
            anon / 2
        );
    }

    #[test]
    fn a_full_zram_spills_to_the_swap_file_behind_it() {
        let memory = Memory::parse(LAPTOP).unwrap();
        let small = Area {
            room: 100 * PAGE,
            ..ZRAM
        };
        let both = Fit::of(memory, Zones::parse(ZONES), GIB, &[FILE, small], QUARTER);
        assert_eq!(both.least - into(FILE, QUARTER).least, 25 * PAGE);
    }

    #[test]
    fn the_cap_is_half_of_memory_less_twice_the_reserve_as_the_kernel_sums_it() {
        // 1,890,532 pages, less 45,004 kept back and 1,024, halved, less
        // twice 262,144.
        assert_eq!(laptop(GIB).most, 397_964 * PAGE);
    }

    #[test]
    fn the_laptop_after_the_rollback_fits_once_the_floors_are_set_aside() {
        let fit = laptop(GIB);
        // In use 4,054,080 KiB, less 15,369 pages waiting on the processors'
        // lists, 2,006,276 KiB of cache and 107,788 of slabs, and of 717,008
        // anonymous what zram frees of it, less the 22% it takes.
        assert_eq!(fit.least, 1_352_590_717);
        assert!(fit.fits(), "{fit:?}");
        assert_eq!(
            fit.said(),
            "It fits: at least 1.4 GB of memory has to be saved, and there's room for 1.6 GB."
        );
    }

    #[test]
    fn a_reserve_as_large_as_memory_leaves_no_room() {
        let fit = laptop(Memory::parse(LAPTOP).unwrap().total);
        assert_eq!(fit.most, 0);
        assert!(!fit.fits());
        assert_eq!(
            fit.said(),
            "Not enough memory to hibernate: 1.4 GB of memory can't be freed, and there's room \
             for only 0.0 GB once drivers have theirs. Nothing was changed. Close something you \
             don't need, then hibernate again."
        );
    }

    #[test]
    fn anonymous_memory_swap_has_no_room_for_stays() {
        let memory = Memory::parse(LAPTOP).unwrap();
        let without = Fit::of(memory, Zones::parse(ZONES), GIB, &[], squeeze());
        assert_eq!(without.least - laptop(GIB).least, 571_034_243);
    }
}
