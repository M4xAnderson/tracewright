//! recheck's own reader for the shared NS §14 vector files. It shares no code with verify's
//! harness or with engine's (ADR 0010).

use std::collections::BTreeMap;

use serde::Deserialize;
use tracewright_net::AsyncLink;
use tracewright_net::Fusion;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::OpenNets;
use tracewright_net::Place;
use tracewright_net::QualifiedId;
use tracewright_net::Transition;
use tracewright_net::TransitionWaits;
use tracewright_net::Violation;
use tracewright_net::Wait;
use tracewright_net::WaitReason;
use tracewright_net::WhenFull;

pub fn q(id: &str) -> QualifiedId {
    QualifiedId(id.to_owned())
}

#[derive(Deserialize)]
pub struct NetSpec {
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

fn when_full(text: Option<&str>) -> WhenFull {
    match text {
        None | Some("block") => WhenFull::Block,
        Some("drop") => WhenFull::Drop,
        Some("overflow") => WhenFull::Overflow,
        Some(other) => panic!("unknown when_full {other}"),
    }
}

#[derive(Deserialize)]
struct TransitionSpec {
    id: String,
    #[serde(default)]
    pre: BTreeMap<String, u32>,
    #[serde(default)]
    post: BTreeMap<String, u32>,
}

impl NetSpec {
    pub fn to_net(&self) -> Net {
        let mut net = Net::default();
        for place in &self.places {
            net.places.push(Place {
                id: q(&place.id),
                capacity: place.capacity,
                when_full: when_full(place.when_full.as_deref()),
            });
            if place.initial > 0 {
                net.initial.0.insert(q(&place.id), place.initial);
            }
        }
        for transition in &self.transitions {
            let mut t = Transition {
                id: q(&transition.id),
                pre: BTreeMap::new(),
                post: BTreeMap::new(),
            };
            for (place, weight) in &transition.pre {
                t.pre.insert(q(place), *weight);
            }
            for (place, weight) in &transition.post {
                t.post.insert(q(place), *weight);
            }
            net.transitions.push(t);
        }
        net
    }
}

#[derive(Deserialize)]
pub struct PropertyFile {
    pub nets: BTreeMap<String, NetSpec>,
    pub vector: Vec<PropertyVector>,
}

#[derive(Deserialize)]
pub struct PropertyVector {
    pub id: String,
    pub net: String,
    property: Option<String>,
    pub expect: String,
    pub markings: Option<u64>,
    pub stored: Option<u64>,
    pub fired: Option<Vec<String>>,
    pub waiting: Option<Vec<WaitingSpec>>,
    overflow: Option<OverflowSpec>,
    pub drop_places: Option<Vec<String>>,
    pub place: Option<String>,
}

#[derive(Deserialize)]
struct OverflowSpec {
    place: String,
    capacity: u32,
    count: u32,
}

impl PropertyVector {
    pub fn property(&self) -> NetProperty {
        match self.property.as_deref() {
            None | Some("deadlock-free") => NetProperty::DeadlockFree,
            Some("no-capacity-exceeded") => NetProperty::CapacityNeverExceeded,
            Some(other) => panic!("{}: unknown property {other}", self.id),
        }
    }

    /// The violation a disproved vector expects.
    pub fn violation(&self) -> Violation {
        match (&self.waiting, &self.overflow) {
            (Some(spec), None) => Violation::Deadlock {
                waiting: waiting(spec),
            },
            (None, Some(spec)) => Violation::Overflow {
                place: q(&spec.place),
                capacity: spec.capacity,
                count: spec.count,
            },
            _ => panic!("{}: a disproved vector has waiting or overflow", self.id),
        }
    }
}

#[derive(Deserialize)]
pub struct WaitingSpec {
    transition: String,
    waits: Vec<WaitSpec>,
}

#[derive(Deserialize)]
struct WaitSpec {
    place: String,
    needs: Option<[u32; 2]>,
    full: Option<[u32; 2]>,
}

pub fn waiting(specs: &[WaitingSpec]) -> Vec<TransitionWaits> {
    let mut out = Vec::new();
    for spec in specs {
        let mut waits = Vec::new();
        for wait in &spec.waits {
            let reason = if let Some([needed, holds]) = wait.needs {
                WaitReason::NeedsTokens { needed, holds }
            } else if let Some([capacity, would_hold]) = wait.full {
                WaitReason::OutputFull {
                    capacity,
                    would_hold,
                }
            } else {
                panic!("a wait states needs or full");
            };
            waits.push(Wait {
                place: q(&wait.place),
                reason,
            });
        }
        out.push(TransitionWaits {
            transition: q(&spec.transition),
            waits,
        });
    }
    out
}

pub fn property_vectors() -> PropertyFile {
    toml::from_str(include_str!("../../../net/vectors/ns14-s0-properties.toml"))
        .expect("property vectors parse")
}

#[derive(Deserialize)]
pub struct CompositionFile {
    pub composition: Vec<CompositionVector>,
}

#[derive(Deserialize)]
pub struct CompositionVector {
    pub id: String,
    parts: Vec<NetSpec>,
    #[serde(default)]
    fusions: Vec<FusionSpec>,
    #[serde(default)]
    links: Vec<LinkSpec>,
    pub encoding: Option<String>,
    pub rejected: Option<String>,
    pub deadlock_free: Option<u64>,
}

#[derive(Deserialize)]
struct FusionSpec {
    relationship: String,
    caller: String,
    callee: String,
}

#[derive(Deserialize)]
struct LinkSpec {
    relationship: String,
    from: Vec<String>,
    to: Vec<String>,
    capacity: u32,
    when_full: String,
}

impl CompositionVector {
    pub fn open_nets(&self) -> OpenNets {
        OpenNets {
            parts: self.parts.iter().map(NetSpec::to_net).collect(),
            fusions: self
                .fusions
                .iter()
                .map(|f| Fusion {
                    relationship: f.relationship.clone(),
                    caller: q(&f.caller),
                    callee: q(&f.callee),
                })
                .collect(),
            links: self
                .links
                .iter()
                .map(|l| AsyncLink {
                    relationship: l.relationship.clone(),
                    from: l.from.iter().map(|id| q(id)).collect(),
                    to: l.to.iter().map(|id| q(id)).collect(),
                    capacity: l.capacity,
                    when_full: when_full(Some(&l.when_full)),
                })
                .collect(),
        }
    }
}

pub fn composition_vectors() -> CompositionFile {
    toml::from_str(include_str!(
        "../../../net/vectors/ns14-s0-composition.toml"
    ))
    .expect("composition vectors parse")
}

#[derive(Deserialize)]
pub struct BudgetFile {
    pub nets: BTreeMap<String, NetSpec>,
    pub vector: Vec<BudgetVector>,
}

#[derive(Deserialize)]
pub struct BudgetVector {
    pub id: String,
    pub net: String,
    property: Option<String>,
    pub n: u64,
    #[serde(default)]
    pub cancel: bool,
    pub expect: String,
    pub markings: Option<u64>,
    pub steps: Option<usize>,
}

impl BudgetVector {
    pub fn property(&self) -> NetProperty {
        match self.property.as_deref() {
            None | Some("deadlock-free") => NetProperty::DeadlockFree,
            Some("no-capacity-exceeded") => NetProperty::CapacityNeverExceeded,
            Some(other) => panic!("{}: unknown property {other}", self.id),
        }
    }
}

pub fn budget_vectors() -> BudgetFile {
    toml::from_str(include_str!("../../../net/vectors/ns14-s0-budget.toml"))
        .expect("budget vectors parse")
}
