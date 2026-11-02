//! recheck's own reading of NS §4 (enabling and firing) and §7 (waits), on a dense form of the
//! net: one row of pre and post weights per transition, one column per place.

use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::QualifiedId;
use tracewright_net::TransitionWaits;
use tracewright_net::Wait;
use tracewright_net::WaitReason;
use tracewright_net::WhenFull;

pub(crate) struct Dense {
    /// Place IDs in canonical order; column `p` of every row is this place.
    pub(crate) places: Vec<QualifiedId>,
    capacity: Vec<u64>,
    block: Vec<bool>,
    /// Transition IDs in canonical order; row `t` of `pre` and `post` is this transition.
    pub(crate) transitions: Vec<QualifiedId>,
    pre: Vec<Vec<u64>>,
    post: Vec<Vec<u64>>,
    pub(crate) initial: Vec<u64>,
}

impl Dense {
    /// The dense form of a structurally valid net.
    pub(crate) fn of(net: &Net) -> Dense {
        let mut places: Vec<&tracewright_net::Place> = net.places.iter().collect();
        places.sort_by(|a, b| a.id.cmp(&b.id));
        let column = |id: &QualifiedId| {
            places
                .binary_search_by(|place| place.id.cmp(id))
                .expect("valid net: arcs name places")
        };
        let mut transitions: Vec<&tracewright_net::Transition> = net.transitions.iter().collect();
        transitions.sort_by(|a, b| a.id.cmp(&b.id));
        let mut pre = vec![vec![0; places.len()]; transitions.len()];
        let mut post = vec![vec![0; places.len()]; transitions.len()];
        for (t, transition) in transitions.iter().enumerate() {
            for (place, weight) in &transition.pre {
                pre[t][column(place)] = u64::from(*weight);
            }
            for (place, weight) in &transition.post {
                post[t][column(place)] = u64::from(*weight);
            }
        }
        let mut initial = vec![0; places.len()];
        for (place, count) in &net.initial.0 {
            initial[column(place)] = u64::from(*count);
        }
        Dense {
            capacity: places
                .iter()
                .map(|place| u64::from(place.capacity))
                .collect(),
            block: places
                .iter()
                .map(|place| matches!(place.when_full, WhenFull::Block))
                .collect(),
            places: places.iter().map(|place| place.id.clone()).collect(),
            transitions: transitions
                .iter()
                .map(|transition| transition.id.clone())
                .collect(),
            pre,
            post,
            initial,
        }
    }

    /// The tentative count M(p) − W⁻(p, t) + W⁺(t, p) of NS §4.1, signed because it is also asked
    /// for at places that hold fewer tokens than the pre weight (waits, NS §7).
    fn tentative(&self, m: &[u64], t: usize, p: usize) -> i128 {
        i128::from(m[p]) - i128::from(self.pre[t][p]) + i128::from(self.post[t][p])
    }

    /// NS §3.2: some place holds more tokens than its capacity.
    pub(crate) fn is_overflow(&self, m: &[u64]) -> bool {
        (0..self.places.len()).any(|p| m[p] > self.capacity[p])
    }

    /// NS §4.1 (E1), (E2) and (E3).
    pub(crate) fn enabled(&self, m: &[u64], t: usize) -> bool {
        if self.is_overflow(m) {
            return false;
        }
        for p in 0..self.places.len() {
            if m[p] < self.pre[t][p] {
                return false;
            }
            if self.post[t][p] > 0
                && self.block[p]
                && self.tentative(m, t, p) > i128::from(self.capacity[p])
            {
                return false;
            }
        }
        true
    }

    /// NS §4.2 for block places: M'(p) = M⁺ₜ(p) for every place.
    pub(crate) fn fire(&self, m: &[u64], t: usize) -> Option<Vec<u64>> {
        let mut next = Vec::with_capacity(m.len());
        for ((count, pre), post) in m.iter().zip(&self.pre[t]).zip(&self.post[t]) {
            next.push(count.checked_sub(*pre)?.checked_add(*post)?);
        }
        Some(next)
    }

    /// waits(M, t) of NS §7 for every transition, in canonical order of transition and place.
    pub(crate) fn waiting(&self, m: &[u64]) -> Option<Vec<TransitionWaits>> {
        let mut all = Vec::new();
        for t in 0..self.transitions.len() {
            let mut waits = Vec::new();
            for p in 0..self.places.len() {
                if self.pre[t][p] > 0 && m[p] < self.pre[t][p] {
                    let needed = u32::try_from(self.pre[t][p]).ok()?;
                    let holds = u32::try_from(m[p]).ok()?;
                    waits.push(Wait {
                        place: self.places[p].clone(),
                        reason: WaitReason::NeedsTokens { needed, holds },
                    });
                } else if self.post[t][p] > 0
                    && self.block[p]
                    && self.tentative(m, t, p) > i128::from(self.capacity[p])
                {
                    let capacity = u32::try_from(self.capacity[p]).ok()?;
                    let would_hold = u32::try_from(self.tentative(m, t, p)).ok()?;
                    waits.push(Wait {
                        place: self.places[p].clone(),
                        reason: WaitReason::OutputFull {
                            capacity,
                            would_hold,
                        },
                    });
                }
            }
            all.push(TransitionWaits {
                transition: self.transitions[t].clone(),
                waits,
            });
        }
        Some(all)
    }

    /// The data form of a marking, without places that hold 0 tokens.
    pub(crate) fn to_marking(&self, m: &[u64]) -> Option<Marking> {
        let mut out = Marking::default();
        for (p, count) in m.iter().enumerate() {
            if *count > 0 {
                out.0
                    .insert(self.places[p].clone(), u32::try_from(*count).ok()?);
            }
        }
        Some(out)
    }
}
