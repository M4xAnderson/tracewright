//! Problems: what makes files not load or checks not run (file-format §10.2, §10.3). A problem is
//! never a finding and never a verdict.

use crate::ElementId;
use crate::PropertyId;
use crate::RelationshipId;
use crate::Scope;

/// The kinds of the file-format §10.3 catalog that slice 0 reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProblemKind {
    /// TOML syntax error, or a file that is not UTF-8: the file does not load.
    Parse,
    /// Element-level schema error: the file does not load.
    Schema,
    /// A byte or count limit at the file level (LIM-2): the file does not load.
    TooLarge,
    /// A schema or limit failure inside the behavior model section.
    BehaviorNotLoaded,
    /// L1: a child has no behavior model while another child has one.
    NoBehavior,
    /// L2: a place without `when_full`.
    NoWhenFull,
    /// L3: a `parent`, relationship `to` or arc place that names nothing.
    Dangling,
    /// L3: a binding port that does not exist.
    UnknownPort,
    /// L4: a transition port that no contributing relationship binds.
    Unbound,
    /// L5: a binding names something that is not a transition port.
    NotAPort,
    /// L6: both ports of a pair are in one element.
    BindingSelf,
    /// L8: more than one asynchronous pair on one relationship.
    BindingAsync,
    /// L10: a place and a transition of one element with the same name (D-26).
    NameClash,
    /// L10 (place) or L13 (asynchronous pair): no capacity.
    NoCapacity,
    /// L10: `initial` above `capacity`.
    BadInitial,
    /// L11: a pair that does not connect the relationship's two elements.
    BindingDirection,
    /// L12: two synchronous pairs with the same fused name.
    BindingDuplicate,
    /// L13: `capacity` or `when_full` on a synchronous pair.
    BadBinding,
    /// L3: a relationship ID used twice, or equal to an element ID.
    DuplicateId,
    /// L3: the parent chain loops.
    ParentCycle,
    /// PL E4xx and E114: only that property is not checked.
    Property,
    /// D-25, slice 0 only: a `[final]` table is kept but not checked.
    FinalNotChecked,
}

/// How bad a problem is (file-format §10.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Something does not load or does not get checked.
    Error,
    /// Kept and reported; checks run.
    Warning,
}

/// One problem (file-format §10.2). `message` is the cause in the NS wording, with qualified IDs;
/// the UI renders the Problems list copy from the typed fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// What is wrong.
    pub kind: ProblemKind,
    /// The NS L-rule, PL code or LIM-2, when there is one.
    pub code: Option<&'static str>,
    /// Error or warning.
    pub severity: Severity,
    /// The project file, for example `elements/billing.toml`.
    pub file: Option<String>,
    /// 1-based line, known only for problems found while parsing.
    pub line: Option<u32>,
    /// 1-based column, with `line`.
    pub column: Option<u32>,
    /// The element the problem is about.
    pub element: Option<ElementId>,
    /// The relationship the problem is about.
    pub relationship: Option<RelationshipId>,
    /// A port, as its qualified ID `element/name`.
    pub port: Option<String>,
    /// A place, as its qualified ID `element/name`.
    pub place: Option<String>,
    /// The property the problem is about.
    pub property: Option<PropertyId>,
    /// The cause.
    pub message: String,
}

impl Problem {
    /// An error with no references yet.
    #[must_use]
    pub fn error(kind: ProblemKind, code: Option<&'static str>, message: String) -> Problem {
        Problem {
            kind,
            code,
            severity: Severity::Error,
            file: None,
            line: None,
            column: None,
            element: None,
            relationship: None,
            port: None,
            place: None,
            property: None,
            message,
        }
    }

    /// The problem in `file`.
    #[must_use]
    pub fn in_file(mut self, file: &str) -> Problem {
        self.file = Some(file.to_owned());
        self
    }

    /// The problem about `element`, whose file is `elements/<element>.toml`.
    #[must_use]
    pub fn on_element(mut self, element: &ElementId) -> Problem {
        self.file = Some(element_file(element));
        self.element = Some(element.clone());
        self
    }
}

/// The file of an element (file-format §2.1).
#[must_use]
pub fn element_file(element: &ElementId) -> String {
    format!("elements/{element}.toml")
}

/// What a problem stops (file-format §10.3 "Blocks", §10.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Blocks {
    /// Checks run; the problem is only reported.
    Nothing,
    /// Only the property named by the problem is not checked.
    Property,
    /// Lowering fails with `ModelIncomplete` for these scopes.
    Scopes(Vec<Scope>),
    /// A file that is not valid TOML: its parent cannot be known, so every combined scope is
    /// blocked until it loads (file-format §10.5).
    EveryCombinedScope,
}

impl Blocks {
    /// Whether lowering `scope` must fail because of this problem.
    #[must_use]
    pub fn touches(&self, scope: &Scope) -> bool {
        match self {
            Blocks::Nothing | Blocks::Property => false,
            Blocks::Scopes(scopes) => scopes.contains(scope),
            Blocks::EveryCombinedScope => scope.behavior == crate::Behavior::Combined,
        }
    }
}

/// An element file that did not load (file-format §10.5). Its bytes stay on disk and are never
/// written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileNotLoaded {
    /// Why: a `Parse`, `Schema` or `TooLarge` problem.
    pub problem: Problem,
    /// The `parent` the loader read leniently.
    pub parent: ParentOfNotLoaded,
}

/// The `parent` of an element file that did not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParentOfNotLoaded {
    /// The file is not valid TOML, or its `parent` is not an ID.
    Unknown,
    /// The file has no `parent`.
    TopLevel,
    /// The file names this parent.
    Element(ElementId),
}
