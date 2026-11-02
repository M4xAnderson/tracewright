//! PNML (P/T net, version 2009) for TAPAAL verifypn, following the encoding of NS §18 item 10
//! (D-35, D-71), which spike E1 checked against verifypn 4.2.3
//! (`docs/product/experiments/e1-state-space.md`):
//!
//! - A `block` place p of capacity K gets a complement place p̄ with initial K − M₀(p). A
//!   transition t with net effect d = W⁺(t, p) − W⁻(p, t) on p gets an arc p̄ → t of weight d when
//!   d > 0 and t → p̄ of weight −d when d < 0, so t is enabled only if M(p) − W⁻ + W⁺ ≤ K, which is
//!   (E3) exactly. The textbook complement (mirrored arcs) disables every read arc on a full
//!   place (NS V08, E1 surprise 1).
//! - An `overflow` place stays a plain place; the queries bound it (`crate::tapaal`).
//! - A net with a `drop` place is excluded: no plain P/T encoding without transition copies
//!   reproduces the clamp (NS V12, D-35).
//! - No inhibitor arcs: verifypn 4.2.3 loses a normal arc next to an inhibitor arc between the
//!   same place and transition (E1 surprise 4).
//! - A net with a count near 2³¹ is excluded: verifypn counts tokens in i32 (E1 surprise 3).

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Write as _;

use tracewright_net::Net;
use tracewright_net::Place;
use tracewright_net::QualifiedId;
use tracewright_net::Transition;
use tracewright_net::WhenFull;
use tracewright_net::validate;

/// The largest token count or arc weight verifypn holds: it counts in i32 and stops with "Number
/// of tokens in <place> exceeded 2147483647" above it (E1 surprise 3).
pub const TAPAAL_MAX_COUNT: u64 = 2_147_483_647;

/// Why a net stays out of the TAPAAL corpus (NS §18 item 10). Exclusions are listed by the corpus
/// run, never silently dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Excluded {
    /// The place drops when full, which the encoding does not express (D-35).
    DropPlace {
        /// The smallest-ID drop place of the net.
        place: QualifiedId,
    },
    /// A count this place can reach in the explored region, cap(p) + max W⁺(t, p), or an arc
    /// weight on it, is above [`TAPAAL_MAX_COUNT`].
    CountBeyondTapaal {
        /// The smallest-ID such place of the net.
        place: QualifiedId,
    },
}

/// The PNML ID of a place or transition. PNML IDs are XML names, which allow neither `/` nor
/// `+`: `/` becomes `__` and `+` becomes `.` (the E1 mapping). Slugs never contain `.`, so the
/// IDs the writer adds (`.cmp` complements, `arc.<n>`, `tracewright.net`) stay apart from them;
/// [`write`] asserts that the mapping stays one-to-one on the net it writes.
#[must_use]
pub fn pnml_id(id: &QualifiedId) -> String {
    id.0.replace('/', "__").replace('+', ".")
}

const STRING_WRITE: &str = "writing to a String never fails";

fn complement_id(place: &QualifiedId) -> String {
    format!("{}.cmp", pnml_id(place))
}

/// Writes `net` as a PNML document for verifypn, with places, transitions and arcs in canonical
/// ID order, so the same net always gives the same bytes.
///
/// # Errors
///
/// [`Excluded`] when the encoding cannot express the net: a `drop` place, or a count verifypn
/// cannot hold.
///
/// # Panics
///
/// If `net` is not structurally valid (NS §2.3): the corpus validates every net before it writes
/// one, as engine does before a check.
pub fn write(net: &Net) -> Result<String, Excluded> {
    assert!(
        validate::net(net).is_ok(),
        "pnml::write: the net is not structurally valid (NS §2.3)"
    );
    let mut places: Vec<&Place> = net.places.iter().collect();
    places.sort_by(|a, b| a.id.cmp(&b.id));
    let mut transitions: Vec<&Transition> = net.transitions.iter().collect();
    transitions.sort_by(|a, b| a.id.cmp(&b.id));
    exclusion(&places, &transitions)?;

    let mut out = String::from(concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<pnml xmlns=\"http://www.pnml.org/version-2009/grammar/pnml\">\n",
        "  <net id=\"tracewright.net\" type=\"http://www.pnml.org/version-2009/grammar/ptnet\">\n",
        "    <page id=\"tracewright.page\">\n",
    ));
    let mut ids = BTreeSet::new();
    for place in &places {
        let initial = net.initial.0.get(&place.id).copied().unwrap_or(0);
        write_place(&mut out, &mut ids, &pnml_id(&place.id), initial);
        match place.when_full {
            WhenFull::Block => {
                let room = place
                    .capacity
                    .checked_sub(initial)
                    .expect("a valid net's initial marking is in bounds (NS §2.3 rule 4)");
                write_place(&mut out, &mut ids, &complement_id(&place.id), room);
            }
            WhenFull::Overflow => {}
            WhenFull::Drop => unreachable!("exclusion() rejects nets with a drop place"),
        }
    }
    for transition in &transitions {
        let id = pnml_id(&transition.id);
        assert!(ids.insert(id.clone()), "pnml::write: two IDs map to {id}");
        writeln!(
            out,
            "      <transition id=\"{id}\"><name><text>{id}</text></name></transition>"
        )
        .expect(STRING_WRITE);
    }
    let when_full: BTreeMap<&QualifiedId, WhenFull> =
        places.iter().map(|p| (&p.id, p.when_full)).collect();
    let mut arcs = 0_usize;
    for transition in &transitions {
        for (source, target, weight) in arcs_of(transition, &when_full) {
            writeln!(
                out,
                "      <arc id=\"arc.{arcs}\" source=\"{source}\" target=\"{target}\">\
                 <inscription><text>{weight}</text></inscription></arc>"
            )
            .expect(STRING_WRITE);
            arcs += 1;
        }
    }
    out.push_str("    </page>\n  </net>\n</pnml>\n");
    Ok(out)
}

fn write_place(out: &mut String, ids: &mut BTreeSet<String>, id: &str, initial: u32) {
    assert!(
        ids.insert(id.to_owned()),
        "pnml::write: two IDs map to {id}"
    );
    write!(
        out,
        "      <place id=\"{id}\"><name><text>{id}</text></name>"
    )
    .expect(STRING_WRITE);
    if initial > 0 {
        write!(
            out,
            "<initialMarking><text>{initial}</text></initialMarking>"
        )
        .expect(STRING_WRITE);
    }
    out.push_str("</place>\n");
}

// The first reason in canonical place order, so the same net is always excluded for the same
// reason.
fn exclusion(places: &[&Place], transitions: &[&Transition]) -> Result<(), Excluded> {
    for place in places {
        match place.when_full {
            WhenFull::Drop => {
                return Err(Excluded::DropPlace {
                    place: place.id.clone(),
                });
            }
            WhenFull::Block | WhenFull::Overflow => {}
        }
    }
    for place in places {
        let weights = transitions.iter().flat_map(|t| {
            let pre = t.pre.get(&place.id).copied().unwrap_or(0);
            let post = t.post.get(&place.id).copied().unwrap_or(0);
            [pre, post]
        });
        let largest_post = transitions
            .iter()
            .filter_map(|t| t.post.get(&place.id).copied())
            .max()
            .unwrap_or(0);
        let reachable = u64::from(place.capacity) + u64::from(largest_post);
        let largest_weight = weights.max().map_or(0, u64::from);
        if reachable.max(largest_weight) > TAPAAL_MAX_COUNT {
            return Err(Excluded::CountBeyondTapaal {
                place: place.id.clone(),
            });
        }
    }
    Ok(())
}

// The arcs of one transition as (source, target, weight): per touched place in canonical order,
// the pre arc, the post arc and, for a block place, the net-effect complement arc.
fn arcs_of(
    transition: &Transition,
    when_full: &BTreeMap<&QualifiedId, WhenFull>,
) -> Vec<(String, String, u32)> {
    let id = pnml_id(&transition.id);
    let touched: BTreeSet<&QualifiedId> = transition
        .pre
        .keys()
        .chain(transition.post.keys())
        .collect();
    let mut arcs = Vec::new();
    for place in touched {
        let pre = transition.pre.get(place).copied().unwrap_or(0);
        let post = transition.post.get(place).copied().unwrap_or(0);
        if pre > 0 {
            arcs.push((pnml_id(place), id.clone(), pre));
        }
        if post > 0 {
            arcs.push((id.clone(), pnml_id(place), post));
        }
        let kind = when_full
            .get(place)
            .expect("a valid net's arcs name existing places (NS §2.3 rule 2)");
        match kind {
            WhenFull::Block => {
                if post > pre {
                    arcs.push((complement_id(place), id.clone(), post - pre));
                } else if pre > post {
                    arcs.push((id.clone(), complement_id(place), pre - post));
                }
            }
            WhenFull::Overflow => {}
            WhenFull::Drop => unreachable!("exclusion() rejects nets with a drop place"),
        }
    }
    arcs
}
