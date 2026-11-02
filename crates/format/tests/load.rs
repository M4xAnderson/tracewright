//! `format::load` on in-memory project files (file-format §2, §3, §6).

use tracewright_format::LoadError;
use tracewright_format::ProjectFile;
use tracewright_model::Behavior;
use tracewright_model::BehaviorModel;
use tracewright_model::Binding;
use tracewright_model::ElementId;
use tracewright_model::ElementKind;
use tracewright_model::Name;
use tracewright_model::PropertyKind;
use tracewright_model::RelationshipId;
use tracewright_model::WhenFull;

fn file(path: &str, text: &str) -> ProjectFile {
    ProjectFile {
        path: path.to_owned(),
        bytes: text.as_bytes().to_vec(),
    }
}

const ARCH: &str = "format = 1\n";

const SYS: &str = r#"format = 1
kind = "software_system"
name = "Worker pool"

[[property]]
id = "no-deadlock"
behavior = "combined"
kind = "deadlock_free"

[final]
combined = "c/idle = 3"
"#;

const CLIENTS: &str = r#"format = 1
kind = "container"
name = "Clients"
parent = "sys"

[petri_net.places.idle]
capacity = 3
when_full = "block"
initial = 3

[petri_net.places.running]
capacity = 3

[petri_net.transitions.start]
port = true
pre = { idle = 1 }
post = { running = 1 }

[relationships.r1]
to = "w"
meaning = "takes a worker from"
interaction = "synchronous"

[[relationships.r1.binding]]
mode = "sync"
from = "c/start"
to = "w/acquire"
"#;

fn id(text: &str) -> ElementId {
    ElementId::parse(text).expect("a valid ID")
}

#[test]
fn a_project_loads_into_the_model() {
    let model = tracewright_format::load(&[
        file("arch.toml", ARCH),
        file("elements/sys.toml", SYS),
        file("elements/c.toml", CLIENTS),
    ])
    .unwrap();

    let sys = &model.elements[&id("sys")];
    assert_eq!(sys.kind, ElementKind::SoftwareSystem);
    assert_eq!(sys.parent, None);
    assert_eq!(sys.properties.len(), 1);
    assert_eq!(sys.properties[0].behavior, Behavior::Combined);
    assert_eq!(sys.properties[0].kind, PropertyKind::DeadlockFree);
    assert_eq!(sys.final_predicates.combined.as_deref(), Some("c/idle = 3"));

    let clients = &model.elements[&id("c")];
    assert_eq!(clients.name, "Clients");
    assert_eq!(clients.parent, Some(id("sys")));
    let Some(BehaviorModel::PetriNet(net)) = &clients.behavior else {
        panic!("c has a Petri net")
    };
    let idle = &net.places[&Name::parse("idle").unwrap()];
    assert_eq!(
        (idle.capacity, idle.when_full, idle.initial, idle.port),
        (Some(3), Some(WhenFull::Block), 3, false)
    );
    let running = &net.places[&Name::parse("running").unwrap()];
    assert_eq!(
        (running.when_full, running.initial),
        (None, 0),
        "a missing when_full is a model problem, not a load failure"
    );
    let start = &net.transitions[&Name::parse("start").unwrap()];
    assert!(start.port);
    assert_eq!(start.pre[&Name::parse("idle").unwrap()], 1);

    let r1 = &model.relationships[&RelationshipId::parse("r1").unwrap()];
    assert_eq!((r1.source.as_str(), r1.target.as_str()), ("c", "w"));
    assert_eq!(r1.bindings.len(), 1);
    assert_eq!(r1.bindings[0].binding, Binding::Sync);
    assert_eq!(
        (
            r1.bindings[0].from.element.as_str(),
            r1.bindings[0].from.name.as_str()
        ),
        ("c", "start")
    );
    assert_eq!(
        (
            r1.bindings[0].to.element.as_str(),
            r1.bindings[0].to.name.as_str()
        ),
        ("w", "acquire")
    );
}

#[test]
fn files_outside_the_project_set_are_ignored() {
    let model = tracewright_format::load(&[
        file("arch.toml", ARCH),
        file("elements/sys.toml", SYS),
        file("README.md", "not toml"),
        file("elements/.pn-0123abcd.tmp", "junk"),
        file("elements/notes.txt", "junk"),
        file("layouts/sys.toml", "format = 1\n[position]\nx = 1\ny = 2\n"),
    ])
    .unwrap();
    assert_eq!(model.elements.len(), 1);
}

#[test]
fn a_folder_without_arch_toml_is_not_a_project() {
    assert_eq!(
        tracewright_format::load(&[file("elements/sys.toml", SYS)]),
        Err(LoadError::NotAProject)
    );
}

#[test]
fn a_newer_format_does_not_open() {
    let error = tracewright_format::load(&[file("arch.toml", "format = 2\n")]).unwrap_err();
    assert_eq!(
        error,
        LoadError::VersionTooNew {
            path: "arch.toml".to_owned(),
            found: 2,
            supported: 1
        }
    );
}

#[test]
fn a_missing_format_key_is_named() {
    let error = tracewright_format::load(&[
        file("arch.toml", ARCH),
        file("elements/sys.toml", "kind = \"container\"\n"),
    ])
    .unwrap_err();
    assert_eq!(
        error,
        LoadError::FormatMissing {
            path: "elements/sys.toml".to_owned()
        }
    );
}

#[test]
fn an_unknown_key_names_its_file() {
    let text = CLIENTS.replace("when_full = \"block\"", "when_ful = \"block\"");
    let error =
        tracewright_format::load(&[file("arch.toml", ARCH), file("elements/c.toml", &text)])
            .unwrap_err();
    let LoadError::Schema { path, message } = error else {
        panic!("{error:?}")
    };
    assert_eq!(path, "elements/c.toml");
    assert!(message.contains("when_ful"), "{message}");
}

#[test]
fn an_unknown_when_full_value_does_not_load() {
    let text = CLIENTS.replace("when_full = \"block\"", "when_full = \"wait\"");
    let error =
        tracewright_format::load(&[file("arch.toml", ARCH), file("elements/c.toml", &text)])
            .unwrap_err();
    assert!(matches!(error, LoadError::Schema { .. }), "{error:?}");
}

#[test]
fn a_file_stem_that_is_not_an_id_does_not_load() {
    let error =
        tracewright_format::load(&[file("arch.toml", ARCH), file("elements/Sys.toml", SYS)])
            .unwrap_err();
    assert!(matches!(error, LoadError::InvalidId { .. }), "{error:?}");
}

#[test]
fn a_file_above_the_byte_limit_is_refused_before_parsing() {
    let huge = "#".repeat(tracewright_format::ELEMENT_FILE_BYTES_MAX + 1);
    let error =
        tracewright_format::load(&[file("arch.toml", ARCH), file("elements/sys.toml", &huge)])
            .unwrap_err();
    assert!(matches!(error, LoadError::TooLarge { .. }), "{error:?}");
}

#[test]
fn a_relationship_id_shares_the_namespace_of_elements() {
    let text = CLIENTS.replace("relationships.r1", "relationships.sys");
    let error = tracewright_format::load(&[
        file("arch.toml", ARCH),
        file("elements/sys.toml", SYS),
        file("elements/c.toml", &text),
    ])
    .unwrap_err();
    assert_eq!(
        error,
        LoadError::IdTaken {
            id: "sys".to_owned()
        }
    );
}

#[test]
fn a_binding_port_must_be_qualified() {
    let text = CLIENTS.replace("from = \"c/start\"", "from = \"start\"");
    let error =
        tracewright_format::load(&[file("arch.toml", ARCH), file("elements/c.toml", &text)])
            .unwrap_err();
    assert!(matches!(error, LoadError::InvalidPort { .. }), "{error:?}");
}

#[test]
fn a_bom_is_stripped() {
    let text = format!("\u{feff}{SYS}");
    assert!(
        tracewright_format::load(&[file("arch.toml", ARCH), file("elements/sys.toml", &text)])
            .is_ok()
    );
}
