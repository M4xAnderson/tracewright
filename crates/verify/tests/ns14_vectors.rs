//! `verify::check` on its own against the NS §14.1 slice 0 vectors with block places and
//! synchronous fusion (seam B). The harness is this crate's own; `recheck` has a separate one.

use std::collections::BTreeMap;

use serde::Deserialize;
use tracewright_net::Budget;
use tracewright_net::Counterexample;
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

#[derive(Deserialize)]
struct File {
    nets: BTreeMap<String, NetSpec>,
    vector: Vec<Vector>,
}

#[derive(Deserialize)]
struct NetSpec {
    places: Vec<PlaceSpec>,
    transitions: Vec<TransitionSpec>,
}

#[derive(Deserialize)]
struct PlaceSpec {
    id: String,
    capacity: u32,
    #[serde(default)]
    initial: u32,
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
    expect: String,
    markings: Option<u64>,
    stored: Option<u64>,
    fired: Option<Vec<String>>,
    waiting: Option<Vec<WaitingSpec>>,
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
                when_full: WhenFull::Block,
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

fn check(net: &Net, max_markings: u64) -> NetVerdict {
    tracewright_verify::check(net, &NetProperty::DeadlockFree, &Budget { max_markings })
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
            check(&nets[&vector.net], ENOUGH),
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
fn every_disproved_vector_has_its_counterexample_and_waiting_list() {
    let (nets, vectors) = vectors();
    for vector in vectors.iter().filter(|v| v.expect == "disproved") {
        let NetVerdict::Disproved(Evidence::Counterexample(Counterexample { steps, violation })) =
            check(&nets[&vector.net], ENOUGH)
        else {
            panic!("{}: not Disproved by a counterexample", vector.id);
        };
        let fired: Vec<&str> = steps.iter().map(|step| step.fired.0.as_str()).collect();
        assert_eq!(
            fired,
            *vector.fired.as_ref().expect("fired"),
            "{}",
            vector.id
        );
        assert_eq!(
            violation,
            Violation::Deadlock {
                waiting: waiting(vector.waiting.as_ref().expect("waiting"))
            },
            "{}",
            vector.id
        );
    }
}

#[test]
fn every_vector_holds_with_exactly_its_stored_count_and_runs_out_with_one_fewer() {
    // NS §10.1 (c) and (d): the budget counts stored markings, M0 included.
    let (nets, vectors) = vectors();
    for vector in vectors.iter().filter(|v| v.expect != "rejected") {
        let needed = vector
            .stored
            .or(vector.markings)
            .expect("stored or markings");
        let net = &nets[&vector.net];
        assert!(
            !matches!(check(net, needed), NetVerdict::Unknown(_)),
            "{} with {needed}",
            vector.id
        );
        assert_eq!(
            check(net, needed - 1),
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
        assert_eq!(
            check(&nets[&vector.net], ENOUGH),
            NetVerdict::Unknown(UnknownReason::InternalError),
            "{}",
            vector.id
        );
    }
}

#[test]
fn every_step_carries_the_marking_after_it() {
    // V24: start, fail, start, fail from c/idle = 3, w/free = 2.
    let (nets, _) = vectors();
    let NetVerdict::Disproved(Evidence::Counterexample(cx)) = check(&nets["POOL_LEAK"], ENOUGH)
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
