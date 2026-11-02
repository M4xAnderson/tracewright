//! What the view carries for the canvas and the behavior model pane (flows §4.3–§4.5): positions
//! from the per-element layout files (file-format §7, ADR 0024), element kinds, ports through the
//! behavior model, and the binding pairs of each relationship.

use tracewright_engine::dto::ArcView;
use tracewright_engine::dto::BehaviorView;
use tracewright_engine::dto::BindingMode;
use tracewright_engine::dto::BindingView;
use tracewright_engine::dto::ElementKind;
use tracewright_engine::dto::ElementView;
use tracewright_engine::dto::PetriNetView;
use tracewright_engine::dto::PlaceView;
use tracewright_engine::dto::PortRefView;
use tracewright_engine::dto::PositionView;
use tracewright_engine::dto::RelationshipView;
use tracewright_engine::dto::TransitionView;
use tracewright_engine::dto::View;
use tracewright_engine::dto::WhenFull;

use super::support::MemDisk;
use super::support::one_job;
use super::support::open;

fn element<'v>(view: &'v View, id: &str) -> &'v ElementView {
    view.elements
        .iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no element {id} in the view"))
}

const JOB_LAYOUT: &str = "format = 1\n\n[position]\nx = 320\ny = -80\nw = 240\n";

#[test]
fn an_element_is_drawn_where_its_layout_file_puts_it() {
    let view = open(&one_job().put("layouts/job.toml", JOB_LAYOUT)).view();
    assert_eq!(
        element(&view, "job").position,
        Some(PositionView {
            x: 320,
            y: -80,
            w: Some(240),
            h: None,
        })
    );
}

#[test]
fn an_element_without_a_layout_file_is_unplaced() {
    let view = open(&one_job()).view();
    assert_eq!(element(&view, "job").position, None);
}

#[test]
fn a_layout_file_that_does_not_load_leaves_only_its_element_unplaced() {
    let disk = MemDisk::fixture("e1b-opposite-locks")
        .put("layouts/a.toml", "format = 1\n\n[position]\nx = 0\ny = 0\n")
        .put(
            "layouts/b.toml",
            "format = 1\n\n[position]\nx = 2000000\ny = 0\n",
        )
        .put(
            "layouts/l.toml",
            "format = 1\n\n[position]\nx = 10\nwhere = 3\n",
        );
    let view = open(&disk).view();
    assert!(element(&view, "a").position.is_some());
    assert_eq!(element(&view, "b").position, None, "x is out of range");
    assert_eq!(element(&view, "l").position, None, "unknown key");
}

fn relationship<'v>(view: &'v View, id: &str) -> &'v RelationshipView {
    view.relationships
        .iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no relationship {id} in the view"))
}

fn port(element: &str, name: &str) -> PortRefView {
    PortRefView {
        element: element.to_owned(),
        name: name.to_owned(),
    }
}

#[test]
fn an_element_carries_its_kind_and_technology() {
    let disk = one_job().edit(
        "elements/job.toml",
        "name = \"Job\"",
        "name = \"Job\"\ntechnology = \"Rust\"",
    );
    let view = open(&disk).view();
    let job = element(&view, "job");
    assert_eq!(job.kind, ElementKind::Container);
    assert_eq!(job.technology.as_deref(), Some("Rust"));
    let system = view.system.as_ref().expect("one-job has a software system");
    assert_eq!(system.kind, ElementKind::SoftwareSystem);
    assert_eq!(system.technology, None);
}

#[test]
fn an_element_carries_its_petri_net_with_capacities_ports_and_arcs() {
    let disk = one_job()
        .edit(
            "elements/job.toml",
            "[petri_net.places.done]",
            "[petri_net.places.done]\nname = \"Done jobs\"\nport = true",
        )
        .edit(
            "elements/job.toml",
            "[petri_net.transitions.work]",
            "[petri_net.transitions.work]\nport = true",
        );
    let view = open(&disk).view();
    let place = |name: &str, display_name: &str, initial, port| PlaceView {
        name: name.to_owned(),
        display_name: display_name.to_owned(),
        capacity: Some(1),
        when_full: Some(WhenFull::Block),
        initial,
        port,
    };
    let arc = |place: &str| ArcView {
        place: place.to_owned(),
        weight: 1,
    };
    assert_eq!(
        element(&view, "job").behavior,
        Some(BehaviorView::PetriNet(PetriNetView {
            places: vec![
                place("done", "Done jobs", 0, true),
                place("todo", "todo", 1, false)
            ],
            transitions: vec![TransitionView {
                name: "work".to_owned(),
                display_name: "work".to_owned(),
                port: true,
                pre: vec![arc("todo")],
                post: vec![arc("done")],
            }],
        }))
    );
    assert_eq!(
        view.system.expect("one-job has a software system").behavior,
        None
    );
}

#[test]
fn a_relationship_carries_its_synchronous_binding_pairs() {
    let view = open(&MemDisk::fixture("e1c-worker-pool")).view();
    assert_eq!(
        relationship(&view, "r1").bindings,
        [BindingView {
            from: port("c", "start"),
            to: port("w", "acquire"),
            mode: BindingMode::Sync,
        }]
    );
}

#[test]
fn an_asynchronous_binding_carries_its_capacity_and_when_full() {
    let disk = MemDisk::fixture("e1c-worker-pool").edit(
        "elements/c.toml",
        "mode = \"sync\"\nfrom = \"c/finish\"",
        "mode = \"async\"\ncapacity = 3\nfrom = \"c/finish\"",
    );
    let view = open(&disk).view();
    assert_eq!(
        relationship(&view, "r2").bindings[0].mode,
        BindingMode::Async {
            capacity: Some(3),
            when_full: WhenFull::Overflow,
        }
    );
}

// The file-format §7 example, with bend points and behavior layout the canvas does not draw yet.
const FULL_LAYOUT: &str = "format = 1

[position]
x = 320
y = 80
w = 200
h = 120

[relationships.ra1]
points = [[440, 40], [560, 40]]

[behavior.places.idle]
x = 40
y = 0
";

#[test]
fn a_layout_file_with_bend_points_and_behavior_layout_loads_and_orphans_are_ignored() {
    let disk = MemDisk::fixture("e1b-opposite-locks")
        .put("layouts/a.toml", FULL_LAYOUT)
        .put(
            "layouts/gone.toml",
            "format = 1\n\n[position]\nx = 0\ny = 0\n",
        );
    let view = open(&disk).view();
    assert_eq!(
        element(&view, "a").position,
        Some(PositionView {
            x: 320,
            y: 80,
            w: Some(200),
            h: Some(120),
        })
    );
    assert!(view.elements.iter().all(|e| e.id != "gone"));
}
