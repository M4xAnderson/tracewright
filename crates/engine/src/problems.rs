//! Problems in the terms of the IPC contract, and the Unknown "model incomplete" of a scope that a
//! problem blocks (file-format §10.5). A scope's detail line and its Problems entry are two
//! renderings of one cause (§10.2): the line uses qualified IDs and the NS wording, the UI renders
//! the entry from the typed fields.

use tracewright_model::Blocks;
use tracewright_model::Model;
use tracewright_model::Problem;
use tracewright_model::ProblemKind;
use tracewright_model::Scope;
use tracewright_model::Severity;

use crate::dto;
use crate::dto::ProblemDto;
use crate::dto::UnknownDto;

/// Every problem of `model` with what it blocks.
pub(crate) struct Blocking {
    problems: Vec<(Problem, Blocks)>,
}

impl Blocking {
    pub(crate) fn new(model: &Model) -> Blocking {
        let problems = tracewright_model::validate(model)
            .into_iter()
            .map(|problem| {
                let blocks = tracewright_model::blocks(model, &problem);
                (problem, blocks)
            })
            .collect();
        Blocking { problems }
    }

    /// Unknown "model incomplete" with one line per problem that blocks `scope`, ordered by rule,
    /// then by line (NS §6.6), followed by the lines only lowering found (`lowering`, for example
    /// L7); `None` when no problem blocks the scope.
    pub(crate) fn incomplete(&self, scope: &Scope, lowering: &[String]) -> Option<UnknownDto> {
        let mut causes: Vec<&Problem> = self
            .problems
            .iter()
            .filter(|(_, blocks)| blocks.touches(scope))
            .map(|(problem, _)| problem)
            .collect();
        if causes.is_empty() {
            return None;
        }
        causes.sort_by_key(|problem| (rule(problem), detail_line(problem)));
        let mut details: Vec<String> = causes.iter().map(|problem| detail_line(problem)).collect();
        assert_eq!(
            details.len(),
            causes.len(),
            "incomplete: the first lines are one per cause"
        );
        // When a file or a behavior did not load, lowering sees less than the files
        // describe, and its further lines ("w/acquire not found") are consequences, not causes.
        let something_missing = causes.iter().any(|problem| {
            matches!(
                problem.kind,
                ProblemKind::Parse
                    | ProblemKind::Schema
                    | ProblemKind::TooLarge
                    | ProblemKind::BehaviorNotLoaded
            )
        });
        let only_lowering: Vec<&String> = lowering
            .iter()
            .filter(|line| !something_missing && !details.contains(line))
            .collect();
        details.extend(only_lowering.into_iter().cloned());
        Some(UnknownDto::ModelIncomplete {
            details,
            causes: causes.into_iter().map(problem_dto).collect(),
        })
    }

    /// The Problems list.
    pub(crate) fn dtos(&self) -> Vec<ProblemDto> {
        self.problems
            .iter()
            .map(|(problem, _)| problem_dto(problem))
            .collect()
    }
}

// The NS rule number a cause sorts by; a file that did not load comes first.
fn rule(problem: &Problem) -> u32 {
    if problem.kind == ProblemKind::BehaviorNotLoaded {
        return 1;
    }
    problem
        .code
        .and_then(|code| code.strip_prefix('L'))
        .and_then(|number| number.parse().ok())
        .unwrap_or(0)
}

/// The detail line of a cause, with qualified IDs (NS §6.6).
fn detail_line(problem: &Problem) -> String {
    match problem.kind {
        ProblemKind::Parse | ProblemKind::Schema | ProblemKind::TooLarge => {
            let file = problem.file.as_deref().unwrap_or("a project file");
            format!("{file} did not load")
        }
        ProblemKind::BehaviorNotLoaded => match &problem.element {
            Some(element) => format!("{element} has a behavior model that did not load"),
            None => problem.message.clone(),
        },
        ProblemKind::NoBehavior
        | ProblemKind::NoWhenFull
        | ProblemKind::Dangling
        | ProblemKind::UnknownPort
        | ProblemKind::Unbound
        | ProblemKind::NotAPort
        | ProblemKind::BindingSelf
        | ProblemKind::BindingAsync
        | ProblemKind::NameClash
        | ProblemKind::NoCapacity
        | ProblemKind::BadInitial
        | ProblemKind::BindingDirection
        | ProblemKind::BindingDuplicate
        | ProblemKind::BadBinding
        | ProblemKind::DuplicateId
        | ProblemKind::ParentCycle
        | ProblemKind::Property
        | ProblemKind::FinalNotChecked => problem.message.clone(),
    }
}

pub(crate) fn problem_dto(problem: &Problem) -> ProblemDto {
    ProblemDto {
        kind: kind(problem.kind),
        code: problem.code.map(str::to_owned),
        severity: match problem.severity {
            Severity::Error => dto::Severity::Error,
            Severity::Warning => dto::Severity::Warning,
        },
        file: problem.file.clone(),
        line: problem.line,
        column: problem.column,
        element: problem.element.as_ref().map(ToString::to_string),
        relationship: problem.relationship.as_ref().map(ToString::to_string),
        port: problem.port.clone(),
        place: problem.place.clone(),
        property: problem.property.as_ref().map(ToString::to_string),
        message: problem.message.clone(),
    }
}

fn kind(kind: ProblemKind) -> dto::ProblemKind {
    match kind {
        ProblemKind::Parse => dto::ProblemKind::Parse,
        ProblemKind::Schema => dto::ProblemKind::Schema,
        ProblemKind::TooLarge => dto::ProblemKind::TooLarge,
        ProblemKind::BehaviorNotLoaded => dto::ProblemKind::BehaviorNotLoaded,
        ProblemKind::NoBehavior => dto::ProblemKind::NoBehavior,
        ProblemKind::NoWhenFull => dto::ProblemKind::NoWhenFull,
        ProblemKind::Dangling => dto::ProblemKind::Dangling,
        ProblemKind::UnknownPort => dto::ProblemKind::UnknownPort,
        ProblemKind::Unbound => dto::ProblemKind::Unbound,
        ProblemKind::NotAPort => dto::ProblemKind::NotAPort,
        ProblemKind::BindingSelf => dto::ProblemKind::BindingSelf,
        ProblemKind::BindingAsync => dto::ProblemKind::BindingAsync,
        ProblemKind::NameClash => dto::ProblemKind::NameClash,
        ProblemKind::NoCapacity => dto::ProblemKind::NoCapacity,
        ProblemKind::BadInitial => dto::ProblemKind::BadInitial,
        ProblemKind::BindingDirection => dto::ProblemKind::BindingDirection,
        ProblemKind::BindingDuplicate => dto::ProblemKind::BindingDuplicate,
        ProblemKind::BadBinding => dto::ProblemKind::BadBinding,
        ProblemKind::DuplicateId => dto::ProblemKind::DuplicateId,
        ProblemKind::ParentCycle => dto::ProblemKind::ParentCycle,
        ProblemKind::Property => dto::ProblemKind::Property,
        ProblemKind::FinalNotChecked => dto::ProblemKind::FinalNotChecked,
    }
}
