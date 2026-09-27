//! The spec types: what a diagram is made of (docs/SPEC.md). Deserialised
//! from JSON with unknown fields refused, so a misspelt field is an error
//! instead of a silent default.

use serde::Deserialize;

/// A whole diagram.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// The spec format's version. Only `1` exists.
    pub archgram: u32,
    /// The diagram's name, written as the SVG's `<title>`.
    pub title: String,
    /// The whole diagram in prose, written as the SVG's `<desc>`.
    pub description: String,
    #[serde(default)]
    pub direction: Direction,
    #[serde(default)]
    pub card: CardStyle,
    #[serde(default)]
    pub logo: LogoPlace,
    #[serde(default = "default_palette")]
    pub palette: String,
    #[serde(default = "default_true")]
    pub legend: bool,
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub frames: Vec<Frame>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub flows: Vec<Flow>,
    #[serde(default)]
    pub hints: Hints,
}

fn default_palette() -> String {
    "mono".to_owned()
}

fn default_true() -> bool {
    true
}

/// The direction the flow runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    #[default]
    Right,
    Down,
}

/// The card style every node in the diagram uses (DESIGN.md, Layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CardStyle {
    #[default]
    Horizontal,
    Vertical,
}

/// Where a technology logo goes (DESIGN.md, Components: Technology logo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogoPlace {
    #[default]
    Corner,
    Chip,
}

/// One thing in the system.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub kind: Kind,
    pub label: String,
    #[serde(default)]
    pub note: Option<String>,
    /// A Simple Icons slug naming the node's technology.
    #[serde(default)]
    pub tech: Option<String>,
    #[serde(default)]
    pub variant: Variant,
    /// The id of the frame the node sits in.
    #[serde(default)]
    pub frame: Option<String>,
}

/// What a node is. Decides its icon and its category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Service,
    Database,
    Queue,
    Cache,
    Storage,
    Users,
    Model,
    VectorStore,
    Tool,
    Agent,
    File,
    Script,
    Generated,
    Check,
    Browser,
    Mobile,
    Desktop,
}

/// The family a kind belongs to; it decides the hue of the icon's lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Core,
    Ai,
    Build,
    Client,
}

impl Kind {
    /// The kind's category (DESIGN.md, Components: Node card).
    #[must_use]
    pub fn category(self) -> Category {
        match self {
            Kind::Service | Kind::Database | Kind::Queue | Kind::Cache | Kind::Storage | Kind::Users => {
                Category::Core
            }
            Kind::Model | Kind::VectorStore | Kind::Tool | Kind::Agent => Category::Ai,
            Kind::File | Kind::Script | Kind::Generated | Kind::Check => Category::Build,
            Kind::Browser | Kind::Mobile | Kind::Desktop => Category::Client,
        }
    }
}

/// How many of a thing there are, and whether it is ours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Variant {
    #[default]
    Single,
    Multi,
    External,
}

/// A boundary around nodes: a network, a trust zone, a team's service.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub parent: Option<String>,
}

/// Data or a call moving from one node to another.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub style: EdgeStyle,
}

/// Solid for the usual path, dashed for one taken only sometimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeStyle {
    #[default]
    Solid,
    Dashed,
}

/// A path archgram animates, step by step along existing edges.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flow {
    pub name: String,
    pub steps: Vec<String>,
}

/// Optional help for the layout.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Hints {
    #[serde(default)]
    pub first: Vec<String>,
    #[serde(default)]
    pub last: Vec<String>,
    #[serde(default)]
    pub same_layer: Vec<Vec<String>>,
    #[serde(default)]
    pub order: Vec<Vec<String>>,
}
