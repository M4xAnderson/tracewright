//! The TAPAAL differential corpus (NS §18 item 10; D-35, D-71): every corpus net is written as
//! PNML, both explorers decide both slice 0 properties on it, and TAPAAL verifypn 4.2.3 must give
//! the same verdict kind, under its default options and with every reduction off. On every
//! exhaustive Proved, verifypn's state count must equal our marking count: O = ∅ there, so
//! verifypn, which explores past overflow markings, explores exactly R.
//!
//! A disagreement fails the run and is a case to investigate; verifypn's answer never feeds a
//! verdict, and the external engine can be the wrong side (E1 surprise 4).
//!
//! The corpus is the E1 nets (`corpus/e1.toml` plus the vectors E1 shares) and every net of the
//! NS §14 slice 0 property and composition vectors. Run it with `tools/check.sh tapaal`, which
//! downloads the pinned verifypn and passes its path in `VERIFYPN`.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;
use tracewright_interop::pnml;
use tracewright_interop::tapaal;
use tracewright_interop::tapaal::Query;
use tracewright_interop::tapaal::VerdictKind;
use tracewright_net::AsyncLink;
use tracewright_net::Budget;
use tracewright_net::Fusion;
use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::OpenNets;
use tracewright_net::Place;
use tracewright_net::QualifiedId;
use tracewright_net::Technique;
use tracewright_net::Transition;
use tracewright_net::UnknownReason;
use tracewright_net::WhenFull;
use tracewright_net::validate;

const PROPERTY_VECTORS: &str = include_str!("../../net/vectors/ns14-s0-properties.toml");
const COMPOSITION_VECTORS: &str = include_str!("../../net/vectors/ns14-s0-composition.toml");
const E1_NETS: &str = include_str!("../corpus/e1.toml");

const PROPERTIES: [NetProperty; 2] = [
    NetProperty::DeadlockFree,
    NetProperty::CapacityNeverExceeded,
];

// verifypn's defaults (structural reductions, stubborn sets, best-first search), and none of
// them with breadth-first search, so a reduction bug shows up as a disagreement (as in E1).
const OPTION_SETS: [&[&str]; 2] = [&[], &["-r", "0", "-p", "-s", "BFS"]];

// The corpus nets the encoding leaves out (NS §18 item 10), pinned so that no net drops out of
// the comparison unnoticed. The drop nets stay covered by the golden vectors of both explorers,
// and so does the u32 edge (V51–V53).
const EXCLUDED: &[(&str, &str)] = &[
    ("PC_drop", "drop place r"),
    ("SRC_drop", "drop place x/p"),
    ("V11", "drop place x/p"),
    ("V12", "drop place x/p"),
    ("V29", "structurally invalid"),
    ("V51", "count of x/p beyond i32"),
    ("V52", "count of x/p beyond i32"),
    ("V53", "drop place x/p"),
    ("V54", "structurally invalid"),
];

// A generous budget: the largest corpus net has 99 reachable markings.
const MAX_MARKINGS: u64 = 1_000_000;

// The wall time one verifypn run may take. The slowest E1 query took 65 ms; a query that explores
// past overflow markings (a plain `EF deadlock`) can run forever, and must fail, not hang CI.
const VERIFYPN_SECONDS: &str = "60";

#[test]
#[ignore = "runs TAPAAL verifypn: tools/check.sh tapaal downloads it and sets VERIFYPN"]
fn every_comparable_run_agrees_with_verifypn() {
    let verifypn = PathBuf::from(
        env::var_os("VERIFYPN").expect("VERIFYPN names the checked binary from tools/tapaal.sh"),
    );
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("tapaal-corpus");
    fs::create_dir_all(&scratch).expect("target/tmp is writable");

    let mut excluded = Vec::new();
    let mut disagreements = Vec::new();
    let mut runs = 0;
    let mut state_counts = 0;
    for (name, net) in corpus() {
        if validate::net(&net).is_err() {
            excluded.push((name, "structurally invalid".to_owned()));
            continue;
        }
        let model = match pnml::write(&net) {
            Ok(model) => model,
            Err(why) => {
                excluded.push((name, exclusion(&why)));
                continue;
            }
        };
        let dir = scratch.join(&name);
        fs::create_dir_all(&dir).expect("target/tmp is writable");
        let model_path = dir.join("model.pnml");
        write_scratch(&model_path, &model);
        for property in &PROPERTIES {
            let run = Run {
                verifypn: &verifypn,
                name: &name,
                net: &net,
                property,
                dir: &dir,
            };
            runs += 1;
            let outcome = run.compare(&model_path);
            state_counts += usize::from(outcome.counted_states);
            disagreements.extend(outcome.disagreements);
        }
    }

    let expected: Vec<(String, String)> = EXCLUDED
        .iter()
        .map(|(n, w)| ((*n).to_owned(), (*w).to_owned()))
        .collect();
    assert_eq!(excluded, expected, "the excluded corpus nets");
    assert!(
        disagreements.is_empty(),
        "{} of {runs} runs disagree with verifypn:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
    report(runs, state_counts, excluded.len());
}

#[expect(clippy::print_stdout, reason = "the corpus summary for the CI log")]
fn report(runs: usize, state_counts: usize, excluded: usize) {
    println!(
        "TAPAAL corpus: {runs} runs agree with verifypn, {state_counts} state counts match, \
         {excluded} nets excluded"
    );
}

struct Outcome {
    // One line per disagreement.
    disagreements: Vec<String>,
    // Whether the run was an exhaustive Proved, whose marking count was compared.
    counted_states: bool,
}

fn exclusion(why: &pnml::Excluded) -> String {
    match why {
        pnml::Excluded::DropPlace { place } => format!("drop place {}", place.0),
        pnml::Excluded::CountBeyondTapaal { place } => format!("count of {} beyond i32", place.0),
    }
}

// One net and one property, compared under every option set.
struct Run<'a> {
    verifypn: &'a Path,
    name: &'a str,
    net: &'a Net,
    property: &'a NetProperty,
    dir: &'a Path,
}

impl Run<'_> {
    fn compare(&self, model: &Path) -> Outcome {
        let at = format!("{} {:?}", self.name, self.property);
        let budget = Budget {
            max_markings: MAX_MARKINGS,
            ..Budget::default()
        };
        let main = kind(&tracewright_verify::check(self.net, self.property, &budget));
        let second = kind(&tracewright_recheck::explore(
            self.net,
            self.property,
            &budget,
        ));
        let (ours, markings) = match (main, second) {
            (Ok(main), Ok(second)) if main.0 == second.0 => main,
            (main, second) => {
                return Outcome {
                    disagreements: vec![format!(
                        "{at}: verify says {main:?}, recheck says {second:?}"
                    )],
                    counted_states: false,
                };
            }
        };
        let mut disagreements = Vec::new();
        for options in OPTION_SETS {
            let theirs = tapaal::decide(self.net, self.property, |query| {
                let query_path = self.dir.join(format!("{}.xml", query.id()));
                write_scratch(&query_path, &tapaal::query_xml(self.net, query));
                let mut args = options.to_vec();
                args.extend(["-n", "-x", "1"]);
                answer(&verifypn(self.verifypn, &args, model, &query_path)?, query)
            });
            if theirs != Ok(ours) {
                disagreements.push(format!(
                    "{at}: we say {ours:?}, verifypn {options:?} says {theirs:?} ({})",
                    self.dir.display()
                ));
            }
        }
        if let Some(markings) = markings {
            let query_path = self
                .dir
                .join(format!("{}.xml", Query::DeadlockReachable.id()));
            write_scratch(
                &query_path,
                &tapaal::query_xml(self.net, Query::DeadlockReachable),
            );
            let args = ["-n", "-e", "-r", "0", "-p"];
            let states =
                verifypn(self.verifypn, &args, model, &query_path).and_then(|o| states(&o));
            if states != Ok(markings) {
                disagreements.push(format!(
                    "{at}: {markings} markings, verifypn -e says {states:?}"
                ));
            }
        }
        Outcome {
            disagreements,
            counted_states: markings.is_some(),
        }
    }
}

// Our verdict kind and, for an exhaustive Proved, the marking count |R|.
fn kind(verdict: &NetVerdict) -> Result<(VerdictKind, Option<u64>), String> {
    match verdict {
        NetVerdict::Proved {
            technique: Technique::Exhaustive,
            markings,
        } => Ok((VerdictKind::Proved, Some(*markings))),
        NetVerdict::Proved {
            technique: Technique::Certificate(_),
            ..
        } => Ok((VerdictKind::Proved, None)),
        NetVerdict::Disproved(_) => Ok((VerdictKind::Disproved, None)),
        NetVerdict::Unknown(UnknownReason::CapacityExceededFirst { .. }) => {
            Ok((VerdictKind::CapacityExceededFirst, None))
        }
        NetVerdict::Unknown(other) => Err(format!("Unknown {other:?}")),
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "SEC-3: the TAPAAL runner starts verifypn, a pinned dev-only tool, on files it wrote"
)]
fn verifypn(bin: &Path, args: &[&str], model: &Path, query: &Path) -> Result<String, String> {
    // coreutils `timeout`: std has no wait with a time limit, and the corpus runs on Linux only.
    let output = Command::new("timeout")
        .args(["--kill-after=10", VERIFYPN_SECONDS])
        .arg(bin)
        .args(args)
        .arg(model)
        .arg(query)
        .output()
        .map_err(|error| format!("cannot start {}: {error}", bin.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.code() == Some(124) {
        return Err(format!(
            "verifypn {args:?} gave no answer within {VERIFYPN_SECONDS} s"
        ));
    }
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "verifypn {args:?} exited with {}: {stderr}",
            output.status
        ));
    }
    Ok(stdout)
}

// verifypn answers in the MCC format, one line `FORMULA <id> TRUE|FALSE TECHNIQUES ...`.
fn answer(stdout: &str, query: Query) -> Result<bool, String> {
    let prefix = format!("FORMULA {} ", query.id());
    let mut answers = stdout.lines().filter_map(|line| line.strip_prefix(&prefix));
    match (
        answers
            .next()
            .and_then(|rest| rest.split_whitespace().next()),
        answers.next(),
    ) {
        (Some("TRUE"), None) => Ok(true),
        (Some("FALSE"), None) => Ok(false),
        _ => Err(format!("no single answer to {}: {stdout}", query.id())),
    }
}

// `verifypn -e` reports `STATE_SPACE STATES <n> TECHNIQUES ...`.
fn states(stdout: &str) -> Result<u64, String> {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("STATE_SPACE STATES "))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|count| count.parse().ok())
        .ok_or_else(|| format!("no state count: {stdout}"))
}

#[expect(
    clippy::disallowed_methods,
    reason = "FMT-2 guards project files; these are scratch files under target/tmp"
)]
fn write_scratch(path: &Path, contents: &str) {
    fs::write(path, contents).expect("target/tmp is writable");
}

// Every corpus net by name, in name order; a name may appear in only one file.
fn corpus() -> BTreeMap<String, Net> {
    let mut nets = BTreeMap::new();
    let mut add = |name: &str, net: Net| {
        assert!(
            nets.insert(name.to_owned(), net).is_none(),
            "corpus net {name} is defined twice"
        );
    };
    for text in [PROPERTY_VECTORS, E1_NETS] {
        let file: NetFile = toml::from_str(text).expect("a corpus net file parses");
        for (name, spec) in &file.nets {
            add(name, spec.to_net());
        }
    }
    let compositions: CompositionFile =
        toml::from_str(COMPOSITION_VECTORS).expect("the composition vectors parse");
    for vector in compositions.composition {
        // Rejected vectors are not nets: lowering refuses them before any check.
        if vector.rejected.is_none() {
            let net = tracewright_recheck::compose(&vector.open_nets())
                .unwrap_or_else(|error| panic!("{} composes: {error}", vector.id));
            add(&vector.id, net);
        }
    }
    nets
}

#[derive(Deserialize)]
struct NetFile {
    nets: BTreeMap<String, NetSpec>,
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
struct CompositionFile {
    composition: Vec<CompositionVector>,
}

#[derive(Deserialize)]
struct CompositionVector {
    id: String,
    parts: Vec<NetSpec>,
    #[serde(default)]
    fusions: Vec<FusionSpec>,
    #[serde(default)]
    links: Vec<LinkSpec>,
    rejected: Option<String>,
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

fn q(id: &str) -> QualifiedId {
    QualifiedId(id.to_owned())
}

fn when_full(name: Option<&str>) -> WhenFull {
    match name {
        None | Some("block") => WhenFull::Block,
        Some("drop") => WhenFull::Drop,
        Some("overflow") => WhenFull::Overflow,
        Some(other) => panic!("unknown when_full {other}"),
    }
}

impl NetSpec {
    fn to_net(&self) -> Net {
        let arcs = |arcs: &BTreeMap<String, u32>| arcs.iter().map(|(p, w)| (q(p), *w)).collect();
        Net {
            places: self
                .places
                .iter()
                .map(|p| Place {
                    id: q(&p.id),
                    capacity: p.capacity,
                    when_full: when_full(p.when_full.as_deref()),
                })
                .collect(),
            transitions: self
                .transitions
                .iter()
                .map(|t| Transition {
                    id: q(&t.id),
                    pre: arcs(&t.pre),
                    post: arcs(&t.post),
                })
                .collect(),
            initial: Marking(
                self.places
                    .iter()
                    .filter(|p| p.initial > 0)
                    .map(|p| (q(&p.id), p.initial))
                    .collect(),
            ),
        }
    }
}

impl CompositionVector {
    fn open_nets(&self) -> OpenNets {
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
