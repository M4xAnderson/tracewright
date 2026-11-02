//! `recheck::explore` on its own against the NS §14.1 slice 0 vectors with block places and
//! synchronous fusion (seam B), and on the composed C vectors.

use tracewright_net::Budget;
use tracewright_net::Evidence;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::Phase;
use tracewright_net::Technique;
use tracewright_net::UnknownReason;
use tracewright_net::Violation;

fn explore(net: &tracewright_net::Net, max_markings: u64) -> NetVerdict {
    tracewright_recheck::explore(net, &NetProperty::DeadlockFree, &Budget { max_markings })
}

#[test]
fn property_vectors() {
    let file = super::support::property_vectors();
    for vector in &file.vector {
        let net = file.nets[&vector.net].to_net();
        let verdict = explore(&net, 1 << 20);
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
                let fired: Vec<String> = cx.steps.iter().map(|s| s.fired.0.clone()).collect();
                assert_eq!(&fired, vector.fired.as_ref().unwrap(), "{}", vector.id);
                let Violation::Deadlock { waiting } = cx.violation;
                assert_eq!(
                    waiting,
                    super::support::waiting(vector.waiting.as_ref().unwrap()),
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
fn stored_counts_pin_the_budget() {
    let file = super::support::property_vectors();
    for vector in file.vector.iter().filter(|v| v.expect != "rejected") {
        let net = file.nets[&vector.net].to_net();
        let exact = vector.stored.unwrap_or_else(|| vector.markings.unwrap());
        assert!(
            !matches!(explore(&net, exact), NetVerdict::Unknown(_)),
            "{}",
            vector.id
        );
        let short = explore(&net, exact - 1);
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
            explore(&net, 1 << 20),
            NetVerdict::Proved {
                technique: Technique::Exhaustive,
                markings
            },
            "{}",
            vector.id
        );
    }
}
