//! A simulated connection, to hold the one-way controller (src/oneway.rs)
//! to the bar its review set before it shapes anything: a modem's two
//! queues, flows that grow until something drops them, other machines
//! beside this one, a call, and the air's late spells. Each of sixteen
//! connections runs ten minutes clean and again rough, the way a real one
//! is: a download's acknowledgements going up, Wi-Fi's jitter on every
//! sample, the air late mostly on the way down, and three reflectors whose
//! timestamps come in whole ms with a clock offset that drifts.
//!
//! The bar, against the same connection unshaped: a 95th percentile delay
//! of at most 100 ms, or half the unshaped one where that's over 200; at
//! least 75% of what the connection could carry each way; and no direction
//! held under half of that more than 30 s longer than unshaped holds it.

use crate::oneway::{Controller, Ours, Seen};
use crate::rate::float;

const DT: f64 = 0.2;
const CALL: f64 = 1_500.0;
const GREEDY: f64 = f64::INFINITY;
const HOME: [f64; 2] = [11_000.0, 250_000.0];

struct Scenario {
    name: &'static str,
    /// The connection's rates, up and down, in kbit/s, at a time.
    cap: fn(f64) -> [f64; 2],
    /// What this machine wants to move, up and down, at a time.
    wants: fn(f64) -> [f64; 2],
    /// What other machines on the network push, up and down, at a time.
    others: fn(f64) -> [f64; 2],
    /// Whether a call is live at a time.
    call: fn(f64) -> bool,
    /// Whether the air delays the probes at a time, with no queue anywhere.
    air: fn(f64) -> bool,
    /// The modem's buffers, up and down, in ms of the starting rates.
    buffer_ms: [f64; 2],
}

#[derive(Default)]
struct Score {
    delays: Vec<f64>,
    got: [f64; 2],
    could: [f64; 2],
    /// The longest each direction carried under half what it could while
    /// it wanted to move something, in seconds.
    stuck: [f64; 2],
    under: [f64; 2],
}

impl Score {
    fn count(&mut self, i: usize, carried: f64, wanted: bool, could: f64) {
        if !wanted {
            self.under[i] = 0.0;
            return;
        }
        self.got[i] += carried * DT;
        self.could[i] += could * DT;
        self.under[i] = if carried < could * 0.5 {
            self.under[i] + DT
        } else {
            0.0
        };
        self.stuck[i] = self.stuck[i].max(self.under[i]);
    }

    /// The delay at a percentile, the nearest sample's.
    fn delay(&mut self, percent: usize) -> f64 {
        self.delays.sort_by(f64::total_cmp);
        let Some(last) = self.delays.len().checked_sub(1) else {
            return 0.0;
        };
        self.delays[(last * percent + 50) / 100]
    }

    /// What share of what it could carry each way it got, in percent, or
    /// None for a direction that wanted nothing.
    fn shares(&self) -> [Option<f64>; 2] {
        [0, 1].map(|i| (self.could[i] > 0.0).then(|| 100.0 * self.got[i] / self.could[i]))
    }
}

/// Draws the same on every run.
struct Draws(u64);

impl Draws {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        float(self.0 >> 40) / float(1 << 24)
    }

    /// A Wi-Fi sample's jitter: about 4 ms either way, and one sample in
    /// twenty 20 ms late.
    fn jitter(&mut self) -> f64 {
        let (spread, late) = (self.next(), self.next());
        (spread - 0.5) * 8.0 + if late < 0.05 { 20.0 } else { 0.0 }
    }
}

/// The connection under way.
struct Connection<'a> {
    scenario: &'a Scenario,
    rough: bool,
    /// The modem's buffers and what's queued in them, up and down, in kbit.
    buffer: [f64; 2],
    queue: [f64; 2],
    flow: [f64; 2],
    acks: f64,
    /// Each reflector's baseline each way, as the controller's reading
    /// learns it.
    bases: [[f64; 2]; 3],
    noise: Draws,
    rand: Draws,
}

impl Connection<'_> {
    /// Moves one step of traffic under the rates held here, and answers
    /// what each way carried.
    fn carry(&mut self, at: f64, here: [f64; 2], score: &mut Score) -> [f64; 2] {
        let s = self.scenario;
        let (cap, wants, others) = ((s.cap)(at), (s.wants)(at), (s.others)(at));
        let call = if (s.call)(at) { CALL } else { 0.0 };
        let mut carried = [0.0; 2];
        for i in 0..2 {
            // A flow grows each step until something drops it; what the
            // shaper here drops holds it at that rate. A download's
            // acknowledgements go up beside the upload's flows.
            let ack = if self.rough && i == 0 { self.acks } else { 0.0 };
            let room = (here[i] - call - ack).max(0.0);
            self.flow[i] = (self.flow[i].max(100.0) * 1.08)
                .min(wants[i])
                .min(room * 1.02);
            let ours = self.flow[i] + call + ack;
            let offered = ours + others[i];
            self.queue[i] = (self.queue[i] + (offered - cap[i]) * DT).max(0.0);
            let full = self.queue[i] >= self.buffer[i];
            self.queue[i] = self.queue[i].min(self.buffer[i]);
            let delivered = if self.queue[i] > 0.0 { cap[i] } else { offered };
            let share = if offered > 0.0 { ours / offered } else { 0.0 };
            if full {
                self.flow[i] *= 0.7;
            }
            carried[i] = (delivered.min(offered) * share).min(here[i]);
            let could = (cap[i] - others[i]).max(0.0).min(wants[i] + call);
            score.count(i, carried[i], wants[i] > 0.0 || ack > 0.0, could);
        }
        if wants[0] > 0.0 || wants[1] > 0.0 {
            score
                .delays
                .push(1000.0 * (self.queue[0] / cap[0] + self.queue[1] / cap[1]));
        }
        self.acks = carried[1] * 0.025;
        carried
    }

    /// Each way's own delay as the controller reads it, and the gateway's
    /// round trip, which crosses the air but not the modem's queue.
    fn seen(&mut self, at: f64) -> ([f64; 2], f64) {
        let cap = (self.scenario.cap)(at);
        let late = if (self.scenario.air)(at) { 60.0 } else { 0.0 };
        let air = if self.rough {
            [late / 6.0, late * 5.0 / 6.0]
        } else {
            [late / 2.0; 2]
        };
        let queued = [0, 1].map(|i| 1000.0 * self.queue[i] / cap[i] + air[i]);
        let delays = if self.rough {
            self.reflected(at, queued)
        } else {
            queued.map(|q| q + self.noise.next())
        };
        let gateway = late
            + if self.rough {
                self.rand.jitter()
            } else {
                self.noise.next()
            };
        (delays, gateway)
    }

    /// The median of three reflectors' readings, each from timestamps in
    /// whole ms with its own drifting offset, over a baseline that falls at
    /// once and rises slowly, to follow the clocks but not a queue.
    fn reflected(&mut self, at: f64, queued: [f64; 2]) -> [f64; 2] {
        let offset = 37.0 + at / 60.0;
        let mut seen = [[0.0; 3]; 2];
        for (k, apart) in [0.0, 13.0, 26.0].into_iter().enumerate() {
            let skew = offset + apart;
            let out = queued[0] + self.rand.jitter();
            let back = queued[1] + self.rand.jitter();
            let base = &mut self.bases[k];
            for (i, raw) in [(out + skew).round(), (back - skew).round()]
                .into_iter()
                .enumerate()
            {
                base[i] = raw.min(base[i] + 0.01);
                seen[i][k] = raw - base[i];
            }
        }
        seen.map(|mut s| {
            s.sort_by(f64::total_cmp);
            s[1]
        })
    }
}

/// Ten minutes of a connection, shaped by a controller starting from the
/// bases given, or unshaped with none.
fn run(scenario: &Scenario, bases: Option<[Option<f64>; 2]>, rough: bool) -> Score {
    let start = (scenario.cap)(0.0);
    let mut connection = Connection {
        scenario,
        rough,
        buffer: [0, 1].map(|i| start[i] * scenario.buffer_ms[i] / 1000.0),
        queue: [0.0; 2],
        flow: [1_000.0; 2],
        acks: 0.0,
        bases: [[f64::INFINITY; 2]; 3],
        noise: Draws(7),
        rand: Draws(11),
    };
    let mut controller = bases.map(|[up, down]| Controller::new(up, down));
    let mut held = bases.map_or([GREEDY; 2], |b| b.map(|b| b.unwrap_or(GREEDY)));
    let mut score = Score::default();
    let mut at = 0.0;
    while at < 600.0 {
        let carried = connection.carry(at, held, &mut score);
        if let Some(c) = controller.as_mut() {
            let (delays, gateway) = connection.seen(at);
            let limits = c.step(&Seen {
                at_s: at,
                up_delay_ms: Some(delays[0]),
                down_delay_ms: Some(delays[1]),
                gateway_ms: Some(gateway),
                up_kbit: carried[0],
                down_kbit: carried[1],
                up_ours: None,
                vpn_ms: None,
            });
            held = [limits.up, limits.down].map(|l| l.map_or(GREEDY, float));
        }
        at += DT;
    }
    score
}

/// Where a shaped connection misses the bar, against the same connection
/// unshaped.
fn misses(shaped: &mut Score, unshaped: &mut Score) -> Vec<String> {
    let mut missed = Vec::new();
    let (p95, unshaped_p95) = (shaped.delay(95), unshaped.delay(95));
    if p95 > 100.0 && !(unshaped_p95 > 200.0 && p95 <= unshaped_p95 / 2.0) {
        missed.push(format!("p95 {p95:.0} ms, unshaped {unshaped_p95:.0}"));
    }
    for (i, way) in ["up", "down"].into_iter().enumerate() {
        if let Some(share) = shaped.shares()[i]
            && share < 75.0
        {
            missed.push(format!("{way} carried {share:.0}%"));
        }
        let allowed = unshaped.stuck[i] + 30.0;
        if shaped.stuck[i] > allowed {
            missed.push(format!(
                "{way} under half {:.0} s, past {allowed:.0}",
                shaped.stuck[i]
            ));
        }
    }
    missed
}

fn busy(at: f64) -> bool {
    (30.0..570.0).contains(&at)
}

fn greedy_down(at: f64) -> [f64; 2] {
    [0.0, if busy(at) { GREEDY } else { 0.0 }]
}

fn greedy_up(at: f64) -> [f64; 2] {
    [if busy(at) { GREEDY } else { 0.0 }, 0.0]
}

fn greedy_both(at: f64) -> [f64; 2] {
    if busy(at) {
        [GREEDY, GREEDY]
    } else {
        [0.0, 0.0]
    }
}

fn home(name: &'static str, wants: fn(f64) -> [f64; 2]) -> Scenario {
    Scenario {
        name,
        cap: |_| HOME,
        wants,
        others: |_| [0.0, 0.0],
        call: |_| false,
        air: |_| false,
        buffer_ms: [300.0, 100.0],
    }
}

/// A roam's rates: the download slides to 30 over ten seconds, stays, and
/// slides back.
fn roam(at: f64) -> [f64; 2] {
    let down = if at < 200.0 {
        250_000.0
    } else if at < 210.0 {
        250_000.0 - 22_000.0 * (at - 200.0)
    } else if at < 400.0 {
        30_000.0
    } else if at < 410.0 {
        30_000.0 + 22_000.0 * (at - 400.0)
    } else {
        250_000.0
    };
    [11_000.0, down]
}

fn scenarios() -> Vec<Scenario> {
    vec![
        home("home, a download", greedy_down),
        home("home, an upload", greedy_up),
        home("home, both ways", greedy_both),
        Scenario {
            call: |_| true,
            ..home("home, both ways, a call", greedy_both)
        },
        Scenario {
            cap: |at| {
                if (200.0..400.0).contains(&at) {
                    [11_000.0, 40_000.0]
                } else {
                    HOME
                }
            },
            ..home("download, the connection drops to 40 and back", greedy_down)
        },
        Scenario {
            cap: |at| {
                if (200.0..400.0).contains(&at) {
                    [4_000.0, 250_000.0]
                } else {
                    HOME
                }
            },
            ..home("upload, the connection drops to 4 and back", greedy_up)
        },
        Scenario {
            others: |at| {
                if (200.0..400.0).contains(&at) {
                    [0.0, 150_000.0]
                } else {
                    [0.0, 0.0]
                }
            },
            ..home("download, another machine takes 150", greedy_down)
        },
        Scenario {
            air: |at| at % 3.0 < 1.0,
            ..home("download, the air late 1 s in 3", greedy_down)
        },
        Scenario {
            cap: |at| {
                let swing = (at / 30.0).sin();
                [5_000.0 + 3_000.0 * swing, 50_000.0 + 30_000.0 * swing]
            },
            buffer_ms: [500.0, 300.0],
            ..home("a tether swinging 20 to 80, both ways", greedy_both)
        },
        Scenario {
            cap: |_| [40_000.0, 900_000.0],
            buffer_ms: [50.0, 50.0],
            ..home("fibre 40/900, small buffers, both ways", greedy_both)
        },
        Scenario {
            cap: |_| [2_000.0, 12_000.0],
            buffer_ms: [2000.0, 1000.0],
            ..home("DSL 2/12, deep buffers, both ways", greedy_both)
        },
        Scenario {
            air: |at| at % 5.0 < 2.0,
            ..home("download, the air late 2 s in 5", greedy_down)
        },
        Scenario {
            cap: roam,
            ..home("a roam, down slides to 30 and back, both ways", greedy_both)
        },
        Scenario {
            air: |at| at % 5.0 < 2.0,
            ..home("both ways, the air late 2 s in 5", greedy_both)
        },
        // Idle five minutes, then a download, so a controller that starts
        // from a wrong base must find its way back while idle.
        home("idle 5 min, then a download", |at| {
            [
                0.0,
                if (300.0..570.0).contains(&at) {
                    GREEDY
                } else {
                    0.0
                },
            ]
        }),
        Scenario {
            cap: |_| [3_000.0, 10_000.0],
            buffer_ms: [1000.0, 1000.0],
            ..home("a hotel 10/3, both ways", greedy_both)
        },
    ]
}

#[test]
fn the_controller_holds_the_bar_on_every_connection_modelled() {
    // Bases it may start from: the connection's own, nearly; none, as on a new
    // network; and a wrong one, remembered from a bad cut.
    let starts = [
        ("a known base", [Some(9_900.0), Some(225_000.0)]),
        ("no base", [None, None]),
        ("a wrong base", [Some(16_000.0), Some(16_100.0)]),
    ];
    let mut missed = Vec::new();
    for scenario in scenarios() {
        let mut unshaped = run(&scenario, None, false);
        for rough in [false, true] {
            for (start, bases) in starts {
                let mut shaped = run(&scenario, Some(bases), rough);
                for miss in misses(&mut shaped, &mut unshaped) {
                    let how = if rough { "rough" } else { "clean" };
                    missed.push(format!("{}, {how}, {start}: {miss}", scenario.name));
                }
            }
        }
    }
    assert!(missed.is_empty(), "{}", missed.join("\n"));
}

/// Ten minutes of an upload behind a router that queues flow by flow, as
/// the eero does: Joel's one upload, from the second half minute, beside
/// 32 agents' that never stop, the line swinging between what the eero's
/// speed tests read, 11.5 to 12.6 Mbit/s, and another machine's 8 flows
/// for two minutes in the middle. The router shares what it carries
/// equally among the flows that reach it and delays no probe, so held over
/// this machine's share our CAKE holds nothing and Joel gets one share of
/// 33; held under it, CAKE's tins put him first, bulk keeping a sixteenth.
/// Answers what share of the line Joel's upload got, and what share this
/// machine's share of the line was used.
fn flow_queued(shaped: bool) -> (f64, f64) {
    const OURS: f64 = 33.0;
    let line = |at: f64| 12_050.0 + 550.0 * (at / 47.0).sin();
    let others = |at: f64| {
        if (300.0..420.0).contains(&at) {
            8.0
        } else {
            0.0
        }
    };
    let mut controller = Controller::new(None, None);
    let mut noise = Draws(5);
    let mut rate = f64::INFINITY;
    let (mut joel, mut joel_could, mut used, mut could) = (0.0, 0.0, 0.0, 0.0);
    let mut at = 0.0;
    while at < 600.0 {
        let share = line(at) * OURS / (OURS + others(at));
        let queued = rate < share;
        let carried = rate.min(share);
        let joel_now = if queued {
            carried * 15.0 / 16.0
        } else {
            carried / OURS
        };
        if at >= 60.0 {
            joel += joel_now * DT;
            joel_could += share * DT;
            used += carried * DT;
            could += share * DT;
        }
        if shaped {
            let limits = controller.step(&Seen {
                at_s: at,
                up_delay_ms: Some(2.0 + 3.0 * noise.next()),
                down_delay_ms: Some(1.0),
                gateway_ms: Some(3.0 + noise.next()),
                up_kbit: carried,
                down_kbit: 300.0,
                up_ours: Some(Ours {
                    queued,
                    waiting: true,
                    ..Ours::default()
                }),
                vpn_ms: None,
            });
            rate = limits.up.map_or(f64::INFINITY, float);
        }
        at += DT;
    }
    (joel / joel_could, used / could)
}

#[test]
fn behind_a_flow_queued_router_joels_upload_gets_most_of_the_line() {
    let (joel, used) = flow_queued(true);
    assert!(
        joel >= 0.75,
        "Joel's upload got {:.0}% of the line",
        joel * 100.0
    );
    assert!(used >= 0.9, "the line was {:.0}% used", used * 100.0);
    // Unshaped, the router gives him one share of 33.
    let (unshaped, _) = flow_queued(false);
    assert!(unshaped < 0.05, "unshaped, he got {:.0}%", unshaped * 100.0);
}

#[test]
fn an_unshaped_connection_misses_the_bar_where_it_should() {
    // The bar has teeth: unshaped, a full download at home queues its
    // buffer's 100 ms and more, so a controller that limited nothing fails.
    let scenario = home("home, both ways", greedy_both);
    let (mut unshaped, mut again) = (run(&scenario, None, false), run(&scenario, None, false));
    let missed = misses(&mut again, &mut unshaped);
    assert!(missed.iter().any(|m| m.starts_with("p95")), "{missed:?}");
}
