//! `verify::check` on its own against the NS §14.1 slice 0 vectors (seam B): block, drop and
//! overflow places, deadlock-free and no capacity exceeded. The harness is this crate's own;
//! `recheck` has a separate one.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use std::time::Instant;

use serde::Deserialize;
use tracewright_net::Budget;
use tracewright_net::Certificate;
use tracewright_net::Clock;
use tracewright_net::Counterexample;
use tracewright_net::Deadline;
use tracewright_net::Evidence;
use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::Phase;
use tracewright_net::Place;
use tracewright_net::QualifiedId;
use tracewright_net::Technique;
use tracewright_net::Transition;
use tracewright_net::TransitionWaits;
use tracewright_net::UnknownReason;
use tracewright_net::Violation;
use tracewright_net::Wait;
use tracewright_net::WaitReason;
use tracewright_net::WhenFull;

const VECTORS: &str = include_str!("../../net/vectors/ns14-s0-properties.toml");
const BUDGET_VECTORS: &str = include_str!("../../net/vectors/ns14-s0-budget.toml");

#[derive(Deserialize)]
struct File {
    nets: BTreeMap<String, NetSpec>,
    vector: Vec<Vector>,
}

#[derive(Deserialize)]
struct NetSpec {
    places: Vec<PlaceSpec>,
    #[serde(default)]
    transitions: Vec<TransitionSpec>,
}

#[derive(Deserialize)]
struct PlaceSpec {
    id: String,
    capacity: u32,
    #[serde(default)]
    initial: u32,
    when_full: Option<String>,
}

#[derive(Deserialize)]
struct TransitionSpec {
    id: String,
    #[serde(default)]
    pre: BTreeMap<String, u32>,
    #[serde(default)]
    post: BTreeMap<String, u32>,
}

#[derive(Deserialize)]
struct Vector {
    id: String,
    net: String,
    property: Option<String>,
    expect: String,
    markings: Option<u64>,
    stored: Option<u64>,
    fired: Option<Vec<String>>,
    waiting: Option<Vec<WaitingSpec>>,
    overflow: Option<OverflowSpec>,
    drop_places: Option<Vec<String>>,
    place: Option<String>,
}

#[derive(Deserialize)]
struct OverflowSpec {
    place: String,
    capacity: u32,
    count: u32,
}

impl Vector {
    fn property(&self) -> NetProperty {
        match self.property.as_deref() {
            None | Some("deadlock-free") => NetProperty::DeadlockFree,
            Some("no-capacity-exceeded") => NetProperty::CapacityNeverExceeded,
            Some(other) => panic!("{}: unknown property {other}", self.id),
        }
    }

    fn fired(&self) -> Vec<&str> {
        self.fired
            .as_ref()
            .expect("a run has fired")
            .iter()
            .map(String::as_str)
            .collect()
    }
}

#[derive(Deserialize)]
struct WaitingSpec {
    transition: String,
    waits: Vec<WaitSpec>,
}

#[derive(Deserialize)]
struct WaitSpec {
    place: String,
    needs: Option<[u32; 2]>,
    full: Option<[u32; 2]>,
}

fn q(id: &str) -> QualifiedId {
    QualifiedId(id.to_owned())
}

fn build(spec: &NetSpec) -> Net {
    let arcs = |map: &BTreeMap<String, u32>| map.iter().map(|(id, w)| (q(id), *w)).collect();
    Net {
        places: spec
            .places
            .iter()
            .map(|p| Place {
                id: q(&p.id),
                capacity: p.capacity,
                when_full: match p.when_full.as_deref() {
                    None | Some("block") => WhenFull::Block,
                    Some("drop") => WhenFull::Drop,
                    Some("overflow") => WhenFull::Overflow,
                    Some(other) => panic!("unknown when_full {other}"),
                },
            })
            .collect(),
        transitions: spec
            .transitions
            .iter()
            .map(|t| Transition {
                id: q(&t.id),
                pre: arcs(&t.pre),
                post: arcs(&t.post),
            })
            .collect(),
        initial: Marking(
            spec.places
                .iter()
                .filter(|p| p.initial > 0)
                .map(|p| (q(&p.id), p.initial))
                .collect(),
        ),
    }
}

fn waiting(spec: &[WaitingSpec]) -> Vec<TransitionWaits> {
    spec.iter()
        .map(|entry| TransitionWaits {
            transition: q(&entry.transition),
            waits: entry
                .waits
                .iter()
                .map(|wait| {
                    let reason = match (wait.needs, wait.full) {
                        (Some([needed, holds]), None) => WaitReason::NeedsTokens { needed, holds },
                        (None, Some([capacity, would_hold])) => WaitReason::OutputFull {
                            capacity,
                            would_hold,
                        },
                        _ => panic!("a wait has exactly one of needs and full"),
                    };
                    Wait {
                        place: q(&wait.place),
                        reason,
                    }
                })
                .collect(),
        })
        .collect()
}

fn check(net: &Net, property: &NetProperty, max_markings: u64) -> NetVerdict {
    let budget = Budget {
        max_markings,
        ..Budget::default()
    };
    tracewright_verify::check(net, property, &budget)
}

fn vectors() -> (BTreeMap<String, Net>, Vec<Vector>) {
    let file: File = toml::from_str(VECTORS).expect("the vector file parses");
    let nets = file
        .nets
        .iter()
        .map(|(name, spec)| (name.clone(), build(spec)))
        .collect();
    (nets, file.vector)
}

const ENOUGH: u64 = 1_000_000;

#[test]
fn every_proved_vector_is_proved_with_its_marking_count() {
    let (nets, vectors) = vectors();
    for vector in vectors.iter().filter(|v| v.expect == "proved") {
        let markings = vector.markings.expect("a proved vector has markings");
        assert_eq!(
            check(&nets[&vector.net], &vector.property(), ENOUGH),
            NetVerdict::Proved {
                technique: Technique::Exhaustive,
                markings
            },
            "{}",
            vector.id
        );
    }
}

#[test]
fn every_certificate_vector_is_proved_by_no_overflow_place_without_exploring() {
    // B13: no exploration happens, so even a budget of 0 markings is enough.
    let (nets, vectors) = vectors();
    for vector in vectors.iter().filter(|v| v.expect == "certificate") {
        let drop_places = vector
            .drop_places
            .as_ref()
            .expect("a certificate vector has drop_places")
            .iter()
            .map(|id| q(id))
            .collect();
        assert_eq!(
            check(&nets[&vector.net], &vector.property(), 0),
            NetVerdict::Proved {
                technique: Technique::Certificate(Certificate::NoOverflowPlace { drop_places }),
                markings: 0
            },
            "{}",
            vector.id
        );
    }
}

#[test]
fn every_disproved_vector_has_its_counterexample_and_violation() {
    let (nets, vectors) = vectors();
    for vector in vectors.iter().filter(|v| v.expect == "disproved") {
        let NetVerdict::Disproved(Evidence::Counterexample(Counterexample { steps, violation })) =
            check(&nets[&vector.net], &vector.property(), ENOUGH)
        else {
            panic!("{}: not Disproved by a counterexample", vector.id);
        };
        let fired: Vec<&str> = steps.iter().map(|step| step.fired.0.as_str()).collect();
        assert_eq!(fired, vector.fired(), "{}", vector.id);
        let expected = match (&vector.waiting, &vector.overflow) {
            (Some(spec), None) => Violation::Deadlock {
                waiting: waiting(spec),
            },
            (None, Some(spec)) => Violation::Overflow {
                place: q(&spec.place),
                capacity: spec.capacity,
                count: spec.count,
            },
            _ => panic!("{}: waiting or overflow", vector.id),
        };
        assert_eq!(violation, expected, "{}", vector.id);
    }
}

#[test]
fn every_capacity_exceeded_first_vector_names_its_place_and_the_run_to_it() {
    let (nets, vectors) = vectors();
    for vector in vectors
        .iter()
        .filter(|v| v.expect == "capacity-exceeded-first")
    {
        let NetVerdict::Unknown(UnknownReason::CapacityExceededFirst { place, steps }) =
            check(&nets[&vector.net], &vector.property(), ENOUGH)
        else {
            panic!("{}: not capacity exceeded first", vector.id);
        };
        assert_eq!(
            place,
            q(vector.place.as_ref().expect("place")),
            "{}",
            vector.id
        );
        let fired: Vec<&str> = steps.iter().map(|step| step.fired.0.as_str()).collect();
        assert_eq!(fired, vector.fired(), "{}", vector.id);
    }
}

#[test]
fn every_explored_vector_holds_with_exactly_its_stored_count_and_runs_out_with_one_fewer() {
    // NS §10.1 (c) and (d): the budget counts stored markings, overflow markings and M0
    // included (B1, B2, B11, B12 among them).
    let (nets, vectors) = vectors();
    for vector in vectors
        .iter()
        .filter(|v| v.expect != "rejected" && v.expect != "certificate")
    {
        let needed = vector
            .stored
            .or(vector.markings)
            .expect("stored or markings");
        let net = &nets[&vector.net];
        let property = vector.property();
        assert!(
            !matches!(
                check(net, &property, needed),
                NetVerdict::Unknown(UnknownReason::BudgetExhausted { .. })
            ),
            "{} with {needed}",
            vector.id
        );
        assert_eq!(
            check(net, &property, needed - 1),
            NetVerdict::Unknown(UnknownReason::BudgetExhausted {
                phase: Phase::Main,
                markings: needed - 1
            }),
            "{} with {}",
            vector.id,
            needed - 1
        );
    }
}

#[test]
fn a_structurally_invalid_net_is_rejected_as_an_internal_error() {
    let (nets, vectors) = vectors();
    for vector in vectors.iter().filter(|v| v.expect == "rejected") {
        for property in [
            NetProperty::DeadlockFree,
            NetProperty::CapacityNeverExceeded,
        ] {
            assert_eq!(
                check(&nets[&vector.net], &property, ENOUGH),
                NetVerdict::Unknown(UnknownReason::InternalError),
                "{}",
                vector.id
            );
        }
    }
}

#[test]
fn every_step_carries_the_marking_after_it() {
    // V24: start, fail, start, fail from c/idle = 3, w/free = 2.
    let (nets, _) = vectors();
    let NetVerdict::Disproved(Evidence::Counterexample(cx)) =
        check(&nets["POOL_LEAK"], &NetProperty::DeadlockFree, ENOUGH)
    else {
        panic!("V24 is Disproved");
    };
    let counts = |step: usize, place: &str| {
        cx.steps[step]
            .marking
            .0
            .get(&q(place))
            .copied()
            .unwrap_or(0)
    };
    let expected = [(2, 1, 1), (3, 0, 1), (2, 1, 0), (3, 0, 0)];
    for (step, (idle, running, free)) in expected.into_iter().enumerate() {
        assert_eq!(
            (
                counts(step, "c/idle"),
                counts(step, "c/running"),
                counts(step, "w/free")
            ),
            (idle, running, free),
            "step {step}"
        );
    }
}

#[test]
fn a_drop_place_keeps_at_most_its_capacity_after_subtracting_first() {
    // NS §4.2: V12 fires x/t from x/p = 1 to min(1, 1 − 1 + 2) = 1, so M0 is its own successor.
    let (nets, _) = vectors();
    assert_eq!(
        check(&nets["V12"], &NetProperty::DeadlockFree, ENOUGH),
        NetVerdict::Proved {
            technique: Technique::Exhaustive,
            markings: 1
        }
    );
}

#[test]
fn the_overflow_run_records_the_marking_above_capacity() {
    // V16: the fourth produce puts r at 4 of 3; the overflow marking is the last step's marking.
    let (nets, _) = vectors();
    let NetVerdict::Disproved(Evidence::Counterexample(cx)) = check(
        &nets["PC_overflow"],
        &NetProperty::CapacityNeverExceeded,
        ENOUGH,
    ) else {
        panic!("V16 is Disproved");
    };
    let last = cx.steps.last().expect("four steps");
    assert_eq!(last.marking.0.get(&q("r")), Some(&4));
    assert_eq!(last.marking.0.get(&q("p/items")), Some(&1));
}

// NS §14.2: the main phase of a check with budget N gets ⌊N/2⌋ markings.
#[derive(Deserialize)]
struct BudgetFile {
    nets: BTreeMap<String, NetSpec>,
    vector: Vec<BudgetVector>,
}

#[derive(Deserialize)]
struct BudgetVector {
    id: String,
    net: String,
    property: Option<String>,
    n: u64,
    #[serde(default)]
    cancel: bool,
    expect: String,
    markings: Option<u64>,
    steps: Option<usize>,
}

#[test]
fn the_budget_vectors_hold_for_the_main_phase() {
    let file: BudgetFile = toml::from_str(BUDGET_VECTORS).expect("the budget vectors parse");
    for vector in &file.vector {
        let budget = Budget {
            max_markings: vector.n / 2,
            cancel: Arc::new(AtomicBool::new(vector.cancel)),
            ..Budget::default()
        };
        let property = match vector.property.as_deref() {
            None | Some("deadlock-free") => NetProperty::DeadlockFree,
            Some("no-capacity-exceeded") => NetProperty::CapacityNeverExceeded,
            Some(other) => panic!("{}: unknown property {other}", vector.id),
        };
        let verdict =
            tracewright_verify::check(&build(&file.nets[&vector.net]), &property, &budget);
        let id = &vector.id;
        match vector.expect.as_str() {
            "certificate" => assert!(
                matches!(
                    verdict,
                    NetVerdict::Proved {
                        technique: Technique::Certificate(_),
                        markings: 0
                    }
                ),
                "{id}: {verdict:?}"
            ),
            "exhausted" => assert_eq!(
                verdict,
                NetVerdict::Unknown(UnknownReason::BudgetExhausted {
                    phase: Phase::Main,
                    markings: vector.markings.expect("markings"),
                }),
                "{id}"
            ),
            "proved" => assert_eq!(
                verdict,
                NetVerdict::Proved {
                    technique: Technique::Exhaustive,
                    markings: vector.markings.expect("markings"),
                },
                "{id}"
            ),
            "disproved" => {
                let NetVerdict::Disproved(Evidence::Counterexample(cx)) = verdict else {
                    panic!("{id}: not Disproved: {verdict:?}");
                };
                assert_eq!(cx.steps.len(), vector.steps.expect("steps"), "{id}");
            }
            "cancelled" => assert_eq!(
                verdict,
                NetVerdict::Unknown(UnknownReason::Cancelled),
                "{id}"
            ),
            other => panic!("{id}: unknown expectation {other}"),
        }
    }
}

// A fake clock that returns its readings in order and then repeats the last one, so a test can
// say what the explorer sees at its first, second, ... poll without waiting.
#[derive(Debug)]
struct Readings(Mutex<Vec<Instant>>);

impl Readings {
    fn new(mut readings: Vec<Instant>) -> Arc<Readings> {
        readings.reverse();
        Arc::new(Readings(Mutex::new(readings)))
    }
}

impl Clock for Readings {
    fn now(&self) -> Instant {
        let mut readings = self.0.lock().expect("test clock");
        if readings.len() > 1 {
            readings.pop().expect("a reading is left")
        } else {
            readings[0]
        }
    }
}

fn with_deadline(at: Instant, clock: Arc<Readings>, cancel: bool) -> Budget {
    Budget {
        max_markings: ENOUGH,
        deadline: Some(Deadline { at, clock }),
        cancel: Arc::new(AtomicBool::new(cancel)),
    }
}

// `x/p(3000, block)`, `x/t: ∅ → x/p`: a chain of 3,001 markings, one expansion per marking, that
// ends in a deadlock at `x/p = 3000`.
fn chain() -> Net {
    let mut net = Net::default();
    net.places.push(Place {
        id: q("x/p"),
        capacity: 3000,
        when_full: WhenFull::Block,
    });
    net.transitions.push(Transition {
        id: q("x/t"),
        pre: BTreeMap::new(),
        post: BTreeMap::from([(q("x/p"), 1)]),
    });
    net
}

#[test]
fn a_deadline_reached_at_the_first_poll_ends_the_phase_with_the_initial_marking_stored() {
    let (nets, _) = vectors();
    let t0 = Instant::now();
    let budget = with_deadline(t0, Readings::new(vec![t0]), false);
    assert_eq!(
        tracewright_verify::check(&nets["POOL"], &NetProperty::DeadlockFree, &budget),
        NetVerdict::Unknown(UnknownReason::BudgetExhausted {
            phase: Phase::Main,
            markings: 1
        })
    );
}

#[test]
fn a_deadline_not_reached_lets_the_phase_finish() {
    let (nets, _) = vectors();
    let t0 = Instant::now();
    let budget = with_deadline(t0 + Duration::from_secs(1), Readings::new(vec![t0]), false);
    assert_eq!(
        tracewright_verify::check(&nets["POOL"], &NetProperty::DeadlockFree, &budget),
        NetVerdict::Proved {
            technique: Technique::Exhaustive,
            markings: 3
        }
    );
}

#[test]
fn the_cancel_flag_wins_over_a_passed_deadline_at_the_same_poll() {
    let (nets, _) = vectors();
    let t0 = Instant::now();
    let budget = with_deadline(t0, Readings::new(vec![t0]), true);
    assert_eq!(
        tracewright_verify::check(&nets["POOL"], &NetProperty::DeadlockFree, &budget),
        NetVerdict::Unknown(UnknownReason::Cancelled)
    );
}

#[test]
fn a_violation_found_before_the_first_poll_is_reported_although_the_deadline_passed() {
    // V01: the initial marking is a deadlock, discovered before the first expansion.
    let (nets, _) = vectors();
    let t0 = Instant::now();
    let budget = with_deadline(t0, Readings::new(vec![t0]), true);
    assert!(matches!(
        tracewright_verify::check(&nets["ONE"], &NetProperty::DeadlockFree, &budget),
        NetVerdict::Disproved(_)
    ));
}

#[test]
fn the_deadline_is_polled_again_after_1024_expansions() {
    // First poll at expansion 0 (before the deadline), the second at expansion 1024 (after it),
    // when markings 0 to 1024 are stored.
    let t0 = Instant::now();
    let at = t0 + Duration::from_secs(1);
    let clock = Readings::new(vec![t0, at]);
    let budget = with_deadline(at, clock, false);
    assert_eq!(
        tracewright_verify::check(&chain(), &NetProperty::DeadlockFree, &budget),
        NetVerdict::Unknown(UnknownReason::BudgetExhausted {
            phase: Phase::Main,
            markings: 1025
        })
    );
}

#[test]
fn progress_reports_the_stored_markings_at_every_poll() {
    let mut reported = Vec::new();
    let budget = Budget {
        max_markings: ENOUGH,
        ..Budget::default()
    };
    let verdict = tracewright_verify::check_reporting(
        &chain(),
        &NetProperty::DeadlockFree,
        &budget,
        &mut |progress| reported.push(progress.markings),
    );
    assert!(matches!(verdict, NetVerdict::Disproved(_)), "{verdict:?}");
    assert_eq!(reported, [1, 1025, 2049]);
}
