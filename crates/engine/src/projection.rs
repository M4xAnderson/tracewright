//! The element and relationship parts of the view (design §3 projection): what the canvas and the
//! behavior model pane draw, in IDs and file names. Pure functions of the model.

use tracewright_model::BehaviorModel;
use tracewright_model::Binding;
use tracewright_model::Element;
use tracewright_model::Model;
use tracewright_model::PetriNet;
use tracewright_model::PortRef;
use tracewright_model::Relationship;

use crate::dto;

pub(crate) fn element_view(model: &Model, element: &Element) -> dto::ElementView {
    let position = model
        .layouts
        .get(&element.id)
        .and_then(|layout| layout.position)
        .map(|position| dto::PositionView {
            x: position.x,
            y: position.y,
            w: position.w,
            h: position.h,
        });
    dto::ElementView {
        id: element.id.to_string(),
        name: element.name.clone(),
        kind: element_kind(element.kind),
        technology: element.technology.clone(),
        position,
        behavior: element.behavior.as_ref().map(|behavior| match behavior {
            BehaviorModel::PetriNet(net) => dto::BehaviorView::PetriNet(petri_net_view(net)),
        }),
    }
}

pub(crate) fn relationship_view(relationship: &Relationship) -> dto::RelationshipView {
    dto::RelationshipView {
        id: relationship.id.to_string(),
        source: relationship.source.to_string(),
        target: relationship.target.to_string(),
        meaning: relationship.meaning.clone(),
        bindings: relationship
            .bindings
            .iter()
            .map(|pair| dto::BindingView {
                from: port_ref(&pair.from),
                to: port_ref(&pair.to),
                mode: match pair.binding {
                    Binding::Sync => dto::BindingMode::Sync,
                    Binding::Async {
                        capacity,
                        when_full,
                    } => dto::BindingMode::Async {
                        capacity,
                        when_full: when_full_view(when_full),
                    },
                },
            })
            .collect(),
    }
}

fn petri_net_view(net: &PetriNet) -> dto::PetriNetView {
    let arcs = |arcs: &std::collections::BTreeMap<tracewright_model::Name, u32>| {
        arcs.iter()
            .map(|(place, weight)| dto::ArcView {
                place: place.to_string(),
                weight: *weight,
            })
            .collect()
    };
    let view = dto::PetriNetView {
        places: net
            .places
            .iter()
            .map(|(name, place)| dto::PlaceView {
                name: name.to_string(),
                display_name: place.name.clone(),
                capacity: place.capacity,
                when_full: place.when_full.map(when_full_view),
                initial: place.initial,
                port: place.port,
            })
            .collect(),
        transitions: net
            .transitions
            .iter()
            .map(|(name, transition)| dto::TransitionView {
                name: name.to_string(),
                display_name: transition.name.clone(),
                port: transition.port,
                pre: arcs(&transition.pre),
                post: arcs(&transition.post),
            })
            .collect(),
    };
    assert_eq!(
        view.places.len(),
        net.places.len(),
        "petri_net_view: one place view per place"
    );
    assert_eq!(
        view.transitions.len(),
        net.transitions.len(),
        "petri_net_view: one transition view per transition"
    );
    view
}

fn port_ref(port: &PortRef) -> dto::PortRefView {
    dto::PortRefView {
        element: port.element.to_string(),
        name: port.name.to_string(),
    }
}

fn element_kind(kind: tracewright_model::ElementKind) -> dto::ElementKind {
    match kind {
        tracewright_model::ElementKind::Person => dto::ElementKind::Person,
        tracewright_model::ElementKind::SoftwareSystem => dto::ElementKind::SoftwareSystem,
        tracewright_model::ElementKind::Container => dto::ElementKind::Container,
        tracewright_model::ElementKind::Database => dto::ElementKind::Database,
        tracewright_model::ElementKind::Queue => dto::ElementKind::Queue,
        tracewright_model::ElementKind::Component => dto::ElementKind::Component,
        tracewright_model::ElementKind::Code => dto::ElementKind::Code,
    }
}

fn when_full_view(when_full: tracewright_model::WhenFull) -> dto::WhenFull {
    match when_full {
        tracewright_model::WhenFull::Block => dto::WhenFull::Block,
        tracewright_model::WhenFull::Drop => dto::WhenFull::Drop,
        tracewright_model::WhenFull::Overflow => dto::WhenFull::Overflow,
    }
}
