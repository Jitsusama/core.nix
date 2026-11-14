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
//! as swap has room for those. The hook switches zswap off first, so what
//! reclaim swaps out for the image goes to the disk and counts as freed;
//! what zswap's pool already holds stays in memory and is in use.
//! Free pages waiting on each processor's own list count as in use there,
//! since only the free lists mark a page free, but they're the first the
//! image's own allocation takes, so here they're free: on a 1 GiB machine
//! they were a ninth of memory.
//! The floors count as nothing, since the hook sets them aside first
//! (floors.rs says why). Pages the graphics drivers' pool keeps once a
//! buffer is freed, `GPUReclaim`, the pool's shrinker gives back under the
//! same reclaim, so they count as freed too: after a restore, the video
//! memory the driver brought back left 868 MB there on the 8 GiB laptop,
//! which put the sums at the cap with its desktop barely open.
//! `minimum_image_size` leaves them out of its count, but its count only
//! sizes the target; the image's own allocation is what reclaims.

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
    gpu: u64,
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
            // A kernel that doesn't count the pool says nothing of it.
            gpu: kib("GPUReclaim").unwrap_or(0),
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
pub fn swap_room(swaps: &str) -> u64 {
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
    pub fn of(memory: Memory, zones: Zones, reserve: u64, swap_room: u64) -> Self {
        let pages = memory.total.saturating_sub(zones.kept_back) / PAGE;
        let most = (pages.saturating_sub(PAGES_FOR_IO) / 2)
            .saturating_sub(2 * reserve.div_ceil(PAGE))
            * PAGE;
        let used = memory
            .total
            .saturating_sub(memory.free)
            .saturating_sub(zones.parked);
        let taken_back = memory.file + memory.slab + memory.gpu + memory.anon.min(swap_room);
        Self {
            most,
            least: used.saturating_sub(taken_back),
        }
    }

    pub fn fits(self) -> bool {
        self.least <= self.most
    }

    /// What `hibernate-unlock fits` says of an image that fits.
    pub fn said(self) -> String {
        format!(
            "It fits: at least {} of memory has to be saved, and there's room for {}.",
            gb(self.least),
            gb(self.most)
        )
    }

    /// Why an image that doesn't fit falls short, and what to do about it.
    pub fn short(self) -> String {
        format!(
            "{} of memory can't be freed, and there's room for only {} once drivers have \
             theirs. Nothing was changed. Close something you don't need, then hibernate again.",
            gb(self.least),
            gb(self.most)
        )
    }
}

/// Bytes in gigabytes to a tenth, as a person reads a machine's memory.
fn gb(bytes: u64) -> String {
    let tenths = (bytes + 50_000_000) / 100_000_000;
    format!("{}.{} GB", tenths / 10, tenths % 10)
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

    /// Its /proc/swaps then, as it is once zram is gone.
    const SWAPS: &str = "Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n\
        /swap/swapfile                          file\t\t8388604\t\t2476\t\t-1\n";

    /// The laptop's /proc/meminfo with a desktop open just after a restart,
    /// when `fits` refused, saying 1.9 GB couldn't be freed.
    const DESKTOP: &str = "MemTotal:        7562128 kB\nMemFree:         2039200 kB\n\
        Active(anon):     794620 kB\nInactive(anon):        0 kB\n\
        Active(file):          0 kB\nInactive(file):  3215884 kB\n\
        SReclaimable:     406536 kB\n";

    /// The laptop's /proc/meminfo with its desktop open, 20 minutes after a
    /// restore, when `fits` said 1.6 GB had to be saved against room for
    /// 1.6 GB.
    const RESTORED: &str = "MemTotal:        7562124 kB\nMemFree:         2357520 kB\n\
        Active(anon):     364504 kB\nInactive(anon):   210928 kB\n\
        Active(file):     341180 kB\nInactive(file):  2241548 kB\n\
        SReclaimable:     424908 kB\nGPUActive:        216140 kB\n\
        GPUReclaim:       868612 kB\n";

    /// Its /proc/zoneinfo's zones the hour before the desktop was refused,
    /// with only the lines the sums read, each zone's twelve processors'
    /// counts summed into one.
    const ZONES: &str = "Node 0, zone      DMA\n        high     72\n        managed  3841\n\
        protection: (0, 1932, 7369, 7369, 7369)\n              count:    0\n\
        Node 0, zone    DMA32\n        high     9366\n        managed  494655\n\
        protection: (0, 0, 5437, 5437, 5437)\n              count:    4137\n\
        Node 0, zone   Normal\n        high     26360\n        managed  1392036\n\
        protection: (0, 0, 0, 0, 0)\n              count:    11232\n\
        Node 0, zone  Movable\n        high     32\n        managed  0\n\
        protection: (0, 0, 0, 0, 0)\n";

    const GIB: u64 = 1024 * 1024 * 1024;

    fn laptop(reserve: u64) -> Fit {
        let memory = Memory::parse(LAPTOP).unwrap();
        Fit::of(memory, Zones::parse(ZONES), reserve, swap_room(SWAPS))
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
        let zones = Zones::parse(ZONES);
        assert_eq!(zones.parked, (4137 + 11_232) * PAGE);
        let more = Zones {
            parked: zones.parked + 100 * PAGE,
            ..zones
        };
        let fit = |zones| Fit::of(memory, zones, GIB, swap_room(SWAPS));
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
    fn a_meminfo_without_the_graphics_pool_has_none() {
        assert_eq!(Memory::parse(LAPTOP).unwrap().gpu, 0);
        assert_eq!(Memory::parse(RESTORED).unwrap().gpu, 868_612 * 1024);
    }

    #[test]
    fn the_desktop_after_a_restore_fits_with_the_graphics_pool_given_back() {
        let memory = Memory::parse(RESTORED).unwrap();
        let fit = Fit::of(memory, Zones::parse(ZONES), GIB, swap_room(SWAPS));
        // In use 5,204,604 KiB, less 15,369 pages waiting on the
        // processors' lists (not read then), 2,582,728 KiB of cache, 424,908
        // of slabs, 868,612 in the graphics pool and 575,432 anonymous.
        assert_eq!(fit.least, 708_042_752);
        assert_eq!(
            fit.said(),
            "It fits: at least 0.7 GB of memory has to be saved, and there's room for 1.6 GB."
        );
    }

    #[test]
    fn a_meminfo_missing_a_line_is_not_read() {
        assert_eq!(Memory::parse("MemTotal: 1 kB\nMemFree: 1 kB\n"), None);
    }

    #[test]
    fn the_room_is_what_every_swap_area_has_left() {
        let swaps = "Filename Type Size Used Priority\n\
                     /swap/swapfile file 8388604 2476 -1\n/swap/other file 100 40 -2\n";
        assert_eq!(swap_room(swaps), (8_388_604 - 2476 + 60) * 1024);
        assert_eq!(swap_room("Filename Type Size Used Priority\n"), 0);
    }

    #[test]
    fn the_desktop_that_was_refused_fits_with_its_anonymous_memory_on_the_disk() {
        let memory = Memory::parse(DESKTOP).unwrap();
        let fit = Fit::of(memory, Zones::parse(ZONES), GIB, swap_room(SWAPS));
        // In use 5,522,928 KiB, less 15,369 pages waiting on the
        // processors' lists (an hour before; they weren't read then),
        // 3,215,884 KiB of cache, 406,536 of slabs and 794,620 anonymous.
        assert_eq!(fit.least, 1_069_477_888);
        assert_eq!(
            fit.said(),
            "It fits: at least 1.1 GB of memory has to be saved, and there's room for 1.6 GB."
        );
    }

    #[test]
    fn anonymous_memory_past_the_swaps_room_stays() {
        let memory = Memory::parse(LAPTOP).unwrap();
        let zones = Zones::parse(ZONES);
        let room = |room| Fit::of(memory, zones, GIB, room).least;
        assert_eq!(room(100 * PAGE) - room(8 * GIB), memory.anon - 100 * PAGE);
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
        // lists, 2,006,276 KiB of cache, 107,788 of slabs and 717,008
        // anonymous.
        assert_eq!(fit.least, 1_189_408_768);
        assert!(fit.fits(), "{fit:?}");
        assert_eq!(
            fit.said(),
            "It fits: at least 1.2 GB of memory has to be saved, and there's room for 1.6 GB."
        );
    }

    #[test]
    fn a_reserve_as_large_as_memory_leaves_no_room() {
        let fit = laptop(Memory::parse(LAPTOP).unwrap().total);
        assert_eq!(fit.most, 0);
        assert!(!fit.fits());
        assert_eq!(
            fit.short(),
            "1.2 GB of memory can't be freed, and there's room for only 0.0 GB once drivers \
             have theirs. Nothing was changed. Close something you don't need, then hibernate \
             again."
        );
    }

    #[test]
    fn anonymous_memory_swap_has_no_room_for_stays() {
        let memory = Memory::parse(LAPTOP).unwrap();
        let without = Fit::of(memory, Zones::parse(ZONES), GIB, 0);
        assert_eq!(without.least - laptop(GIB).least, 734_216_192);
    }
}
