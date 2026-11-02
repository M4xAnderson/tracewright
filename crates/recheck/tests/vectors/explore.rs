//! `recheck::explore` on its own against the NS §14.1 slice 0 vectors (seam B): block, drop and
//! overflow places, deadlock-free and no capacity exceeded; and on the composed C vectors.

use tracewright_net::Budget;
use tracewright_net::Evidence;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::Phase;
use tracewright_net::Technique;
use tracewright_net::UnknownReason;

fn explore(net: &tracewright_net::Net, property: &NetProperty, max_markings: u64) -> NetVerdict {
    let budget = Budget {
        max_markings,
        ..Budget::default()
    };
    tracewright_recheck::explore(net, property, &budget)
}

fn fired(steps: &[tracewright_net::Step]) -> Vec<String> {
    steps.iter().map(|s| s.fired.0.clone()).collect()
}

#[test]
fn property_vectors() {
    let file = super::support::property_vectors();
    for vector in file.vector.iter().filter(|v| v.expect != "certificate") {
        let net = file.nets[&vector.net].to_net();
        let verdict = explore(&net, &vector.property(), 1 << 20);
        match vector.expect.as_str() {
            "proved" => assert_eq!(
                verdict,
                NetVerdict::Proved {
                    technique: Technique::Exhaustive,
                    markings: vector.markings.unwrap()
                },
                "{}",
                vector.id
            ),
            "disproved" => {
                let NetVerdict::Disproved(Evidence::Counterexample(cx)) = verdict else {
                    panic!("{}: {verdict:?}", vector.id);
                };
                assert_eq!(
                    &fired(&cx.steps),
                    vector.fired.as_ref().unwrap(),
                    "{}",
                    vector.id
                );
                assert_eq!(cx.violation, vector.violation(), "{}", vector.id);
            }
            "capacity-exceeded-first" => {
                let NetVerdict::Unknown(UnknownReason::CapacityExceededFirst { place, steps }) =
                    verdict
                else {
                    panic!("{}: {verdict:?}", vector.id);
                };
                assert_eq!(&place.0, vector.place.as_ref().unwrap(), "{}", vector.id);
                assert_eq!(
                    &fired(&steps),
                    vector.fired.as_ref().unwrap(),
                    "{}",
                    vector.id
                );
            }
            "rejected" => assert_eq!(
                verdict,
                NetVerdict::Unknown(UnknownReason::InternalError),
                "{}",
                vector.id
            ),
            other => panic!("unknown expectation {other}"),
        }
    }
}

#[test]
fn a_net_without_an_overflow_place_explores_to_the_same_answer_the_certificate_gives() {
    // recheck never emits a certificate; on a certificate vector its exploration finds no
    // overflow marking, so no capacity exceeded is Proved by exhaustion.
    let file = super::support::property_vectors();
    for vector in file.vector.iter().filter(|v| v.expect == "certificate") {
        let net = file.nets[&vector.net].to_net();
        let verdict = explore(&net, &NetProperty::CapacityNeverExceeded, 1 << 20);
        assert!(
            matches!(
                verdict,
                NetVerdict::Proved {
                    technique: Technique::Exhaustive,
                    ..
                }
            ),
            "{}: {verdict:?}",
            vector.id
        );
    }
}

#[test]
fn stored_counts_pin_the_budget() {
    let file = super::support::property_vectors();
    for vector in file
        .vector
        .iter()
        .filter(|v| v.expect != "rejected" && v.expect != "certificate")
    {
        let net = file.nets[&vector.net].to_net();
        let property = vector.property();
        let exact = vector.stored.unwrap_or_else(|| vector.markings.unwrap());
        assert!(
            !matches!(
                explore(&net, &property, exact),
                NetVerdict::Unknown(UnknownReason::BudgetExhausted { .. })
            ),
            "{}",
            vector.id
        );
        let short = explore(&net, &property, exact - 1);
        let expected = UnknownReason::BudgetExhausted {
            phase: Phase::Recheck,
            markings: exact - 1,
        };
        assert_eq!(short, NetVerdict::Unknown(expected), "{}", vector.id);
    }
}

#[test]
fn composed_vectors_have_their_marking_counts() {
    for vector in super::support::composition_vectors().composition {
        let Some(markings) = vector.deadlock_free else {
            continue;
        };
        let net = tracewright_recheck::compose(&vector.open_nets()).unwrap();
        assert_eq!(
            explore(&net, &NetProperty::DeadlockFree, 1 << 20),
            NetVerdict::Proved {
                technique: Technique::Exhaustive,
                markings
            },
            "{}",
            vector.id
        );
    }
}
