//! The ID and Name alphabets of file-format §4.1.

use tracewright_model::ElementId;
use tracewright_model::Name;

#[test]
fn ids_are_lowercase_letter_first_slugs_of_at_most_48_characters() {
    for ok in [
        "order-api",
        "order-api-2",
        "a",
        "fe1",
        "api_b",
        &"a".repeat(48),
    ] {
        assert!(ElementId::parse(ok).is_ok(), "{ok}");
    }
    for bad in [
        "",
        "Order",
        "1st",
        "_x",
        "order-",
        "and",
        "con",
        "lpt9",
        "a/b",
        "a b",
        &"a".repeat(49),
    ] {
        assert!(ElementId::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn names_keep_their_case_but_have_no_double_underscore() {
    for ok in ["idle", "has_1", "Ret_A", "_x", "take-lock"] {
        assert!(Name::parse(ok).is_ok(), "{ok}");
    }
    for bad in ["", "1x", "a__b", "x-", "not", "a+b", "a/b"] {
        assert!(Name::parse(bad).is_err(), "{bad}");
    }
}
