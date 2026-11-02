//! `CheckKey` fitness and invariance (NS §11, ADR 0013): every semantic change to the net or the
//! property changes the key; display names, layout, the order of entries in the files and a
//! reload do not.

use super::support::MemDisk;
use super::support::run_checks;

fn locks() -> MemDisk {
    MemDisk::fixture("e1b-opposite-locks")
}

fn key(disk: &MemDisk) -> String {
    run_checks(disk).key("no-deadlock").to_string()
}

#[test]
fn a_key_is_64_lowercase_hex_characters() {
    let key = key(&locks());
    assert_eq!(key.len(), 64);
    assert!(
        key.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "{key}"
    );
}

#[test]
fn every_semantic_change_changes_the_key() {
    let base = key(&locks());
    let mutations = [
        ("capacity", locks().edit("elements/l.toml", "[petri_net.places.x]\ncapacity = 1", "[petri_net.places.x]\ncapacity = 2")),
        ("initial marking", locks().edit("elements/l.toml", "[petri_net.places.y]\ncapacity = 1\nwhen_full = \"block\"\ninitial = 1", "[petri_net.places.y]\ncapacity = 1\nwhen_full = \"block\"")),
        ("arc weight", locks().edit("elements/a.toml", "pre = { idle = 1 }", "pre = { idle = 2 }")),
        ("an extra arc", locks().edit("elements/a.toml", "pre = { has_2 = 1 }", "pre = { has_2 = 1, idle = 1 }")),
        ("lock order (LOCKS(xy))", locks().edit("elements/b.toml", "to = \"l/acq_y\"", "to = \"l/acq_z\"").edit("elements/b.toml", "to = \"l/acq_x\"", "to = \"l/acq_y\"").edit("elements/b.toml", "to = \"l/acq_z\"", "to = \"l/acq_x\"")),
        ("a place ID", locks().edit("elements/a.toml", "[petri_net.places.has_2]", "[petri_net.places.holds_2]").edit("elements/a.toml", "has_2 = 1", "holds_2 = 1")),
        ("a relationship ID", locks().edit("elements/a.toml", "relationships.ra1", "relationships.ra9")),
        ("an extra transition", locks().edit("elements/a.toml", "[petri_net.transitions.take_x]", "[petri_net.transitions.rest]\npre = { idle = 1 }\npost = { idle = 1 }\n\n[petri_net.transitions.take_x]")),
    ];
    for (what, disk) in mutations {
        assert_ne!(key(&disk), base, "{what} must change the key");
    }
}

#[test]
fn display_names_and_descriptions_do_not_change_the_key() {
    let base = key(&locks());
    let renamed = locks()
        .edit(
            "elements/a.toml",
            "name = \"Component A\"",
            "name = \"Billing\"\ndescription = \"Takes x first\"\ntechnology = \"Rust\"",
        )
        .edit(
            "elements/l.toml",
            "[petri_net.places.x]",
            "[petri_net.places.x]\nname = \"lock x\"",
        )
        .edit(
            "elements/a.toml",
            "[relationships.ra1]\nto = \"l\"",
            "[relationships.ra1]\nto = \"l\"\nmeaning = \"locks\"",
        );
    assert_eq!(key(&renamed), base);
}

#[test]
fn layout_files_do_not_change_the_key() {
    let base = key(&locks());
    let laid_out = locks().put(
        "layouts/a.toml",
        "format = 1\n\n[position]\nx = 10\ny = 20\n",
    );
    assert_eq!(key(&laid_out), base);
}

#[test]
fn the_order_of_entries_in_the_files_does_not_change_the_key() {
    let base = key(&locks());
    let reordered = locks().put(
        "elements/l.toml",
        "format = 1\nparent = \"sys\"\nkind = \"container\"\nname = \"Lock service\"\n\n\
         [petri_net.transitions.rel_xy]\nport = true\npost = { y = 1, x = 1 }\n\n\
         [petri_net.transitions.acq_y]\nport = true\npre = { y = 1 }\n\n\
         [petri_net.transitions.acq_x]\nport = true\npre = { x = 1 }\n\n\
         [petri_net.places.y]\ninitial = 1\nwhen_full = \"block\"\ncapacity = 1\n\n\
         [petri_net.places.x]\nwhen_full = \"block\"\ncapacity = 1\ninitial = 1\n",
    );
    assert_eq!(key(&reordered), base);
}

#[test]
fn reloading_the_same_files_gives_the_same_key() {
    assert_eq!(key(&locks()), key(&locks()));
}

#[test]
fn a_final_table_does_not_change_the_key_in_slice_0() {
    // D-25: slice 0 never parses `[final]` and lowers `final none`, so the property is the same.
    let base = key(&locks());
    let with_final = locks().edit(
        "elements/sys.toml",
        "name = \"Two components, two locks\"",
        "name = \"Two components, two locks\"\n\n[final]\ncombined = \"a/idle = 1\"",
    );
    assert_eq!(key(&with_final), base);
}
