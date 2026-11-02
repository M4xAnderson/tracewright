//! Seam B on the engine side: `engine::lower::flatten` against the NS §14.3 composition vectors,
//! and the property of NS §14.3 C16 and design §8: on random bounded `OpenNets`, the engine's
//! flattening and recheck's composition encode to the same bytes, and `verify` on the one agrees
//! with `recheck::explore` on the other.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use proptest::prelude::Strategy;
use proptest::prop_assert;
use proptest::prop_assert_eq;
use proptest::proptest;
use serde::Deserialize;
use tracewright_engine::lower::flatten;
use tracewright_net::Budget;
use tracewright_net::Fusion;
use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::OpenNets;
use tracewright_net::Place;
use tracewright_net::QualifiedId;
use tracewright_net::Transition;
use tracewright_net::WhenFull;
use tracewright_net::encode;

// The engine's own reader for the shared vector file.
#[derive(Deserialize)]
struct Vectors {
    composition: Vec<Vector>,
}

#[derive(Deserialize)]
struct Vector {
    id: String,
    parts: Vec<PartSpec>,
    fusions: Vec<BTreeMap<String, String>>,
    encoding: Option<String>,
}

#[derive(Deserialize)]
struct PartSpec {
    places: Vec<BTreeMap<String, toml::Value>>,
    transitions: Vec<BTreeMap<String, toml::Value>>,
}

fn q(id: &str) -> QualifiedId {
    QualifiedId(id.to_owned())
}

fn int(value: &toml::Value) -> u32 {
    u32::try_from(value.as_integer().expect("an integer")).expect("fits in u32")
}

fn arcs(value: Option<&toml::Value>) -> BTreeMap<QualifiedId, u32> {
    value
        .and_then(toml::Value::as_table)
        .map_or_else(BTreeMap::new, |t| {
            t.iter().map(|(k, v)| (q(k), int(v))).collect()
        })
}

fn open_nets(vector: &Vector) -> OpenNets {
    let parts = vector
        .parts
        .iter()
        .map(|part| Net {
            places: part
                .places
                .iter()
                .map(|p| Place {
                    id: q(p["id"].as_str().expect("test fixture")),
                    capacity: int(&p["capacity"]),
                    when_full: WhenFull::Block,
                })
                .collect(),
            transitions: part
                .transitions
                .iter()
                .map(|t| Transition {
                    id: q(t["id"].as_str().expect("test fixture")),
                    pre: arcs(t.get("pre")),
                    post: arcs(t.get("post")),
                })
                .collect(),
            initial: Marking(
                part.places
                    .iter()
                    .filter_map(|p| {
                        p.get("initial")
                            .map(|i| (q(p["id"].as_str().expect("test fixture")), int(i)))
                    })
                    .collect(),
            ),
        })
        .collect();
    let fusions = vector
        .fusions
        .iter()
        .map(|f| Fusion {
            relationship: f["relationship"].clone(),
            caller: q(&f["caller"]),
            callee: q(&f["callee"]),
        })
        .collect();
    OpenNets { parts, fusions }
}

#[test]
fn flatten_produces_the_exact_encoding_of_every_valid_vector() {
    let vectors: Vectors = toml::from_str(include_str!(
        "../../../net/vectors/ns14-s0-composition.toml"
    ))
    .expect("test fixture");
    let mut checked = 0;
    for vector in vectors.composition.iter().filter(|v| v.encoding.is_some()) {
        let flat = flatten(&open_nets(vector)).unwrap_or_else(|e| panic!("{}: {e}", vector.id));
        assert_eq!(
            String::from_utf8(encode::canonical(&flat)).expect("test fixture"),
            *vector.encoding.as_ref().expect("test fixture"),
            "{}",
            vector.id
        );
        checked += 1;
    }
    assert_eq!(checked, 4, "C1, C2, C3 and C21");
}

// A random bounded part: element `e<index>` with 1–2 block places of capacity 1–2 and 1–3
// transitions whose arcs have weights 0–2. Small enough that every net explores in well under a
// millisecond, large enough to deadlock, block and fuse in every combination.
fn part(index: usize) -> impl Strategy<Value = Net> {
    let places = proptest::collection::vec((1u32..=2, 0u32..=2), 1..=2);
    places.prop_flat_map(move |places| {
        let n = places.len();
        let transition = (
            proptest::collection::vec(0u32..=2, n),
            proptest::collection::vec(0u32..=2, n),
        );
        (
            proptest::strategy::Just(places),
            proptest::collection::vec(transition, 1..=3),
        )
            .prop_map(move |(places, ts)| {
                let place_id = |p: usize| q(&format!("e{index}/p{p}"));
                let mut net = Net::default();
                for (p, (capacity, initial)) in places.iter().enumerate() {
                    net.places.push(Place {
                        id: place_id(p),
                        capacity: *capacity,
                        when_full: WhenFull::Block,
                    });
                    if (*initial).min(*capacity) > 0 {
                        net.initial.0.insert(place_id(p), (*initial).min(*capacity));
                    }
                }
                for (t, (pre, post)) in ts.iter().enumerate() {
                    let side = |weights: &Vec<u32>| {
                        weights
                            .iter()
                            .enumerate()
                            .filter(|(_, w)| **w > 0)
                            .map(|(p, w)| (place_id(p), *w))
                            .collect()
                    };
                    net.transitions.push(Transition {
                        id: q(&format!("e{index}/t{t}")),
                        pre: side(pre),
                        post: side(post),
                    });
                }
                net
            })
    })
}

// 1–3 parts and up to 4 candidate fusions between transitions of different parts. Candidates
// that would break L6, L7 or L12 are dropped, as lowering would refuse them.
#[expect(
    clippy::similar_names,
    reason = "caller and callee are the terms of NS §6.3"
)]
fn open_nets_strategy() -> impl Strategy<Value = OpenNets> {
    (1usize..=3).prop_flat_map(|count| {
        let parts: Vec<_> = (0..count).map(part).collect();
        let fusions =
            proptest::collection::vec((0..count, 0usize..3, 0..count, 0usize..3, 0usize..2), 0..=4);
        (parts, fusions).prop_map(|(parts, candidates)| {
            let mut fusions = Vec::new();
            let mut callers = BTreeSet::new();
            let mut callees = BTreeSet::new();
            let mut fused = BTreeSet::new();
            for (a, ta, c, tc, r) in candidates {
                if a == c || ta >= parts[a].transitions.len() || tc >= parts[c].transitions.len() {
                    continue;
                }
                let caller = parts[a].transitions[ta].id.clone();
                let callee = parts[c].transitions[tc].id.clone();
                if callees.contains(&caller)
                    || callers.contains(&callee)
                    || !fused.insert((r, ta, a, tc, c))
                {
                    continue;
                }
                callers.insert(caller.clone());
                callees.insert(callee.clone());
                fusions.push(Fusion {
                    relationship: format!("r{r}-{a}-{c}"),
                    caller,
                    callee,
                });
            }
            OpenNets { parts, fusions }
        })
    })
}

proptest! {
    #[test]
    fn both_compositions_encode_equally_and_both_explorers_agree(open in open_nets_strategy()) {
        let flat = flatten(&open).expect("the generator only builds valid OpenNets");
        let own = tracewright_recheck::compose(&open).expect("recheck composes what engine flattens");
        prop_assert_eq!(encode::canonical(&flat), encode::canonical(&own));
        // At most 3 parts × 2 places × capacity 2: 3^6 = 729 markings, far below the budget.
        let budget = Budget { max_markings: 100_000, ..Budget::default() };
        let main = tracewright_verify::check(&flat, &NetProperty::DeadlockFree, &budget);
        let second = tracewright_recheck::explore(&own, &NetProperty::DeadlockFree, &budget);
        prop_assert!(!matches!(main, NetVerdict::Unknown(_)), "{:?}", main);
        prop_assert_eq!(main, second);
    }
}
