//! What `parley` understands: intents and the slots each one may have.
//! Adding an intent = a variant here + its name and slots + templates
//! for training, then retrain.

/// What the user wants to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Intent {
    /// `new 20m timer then notify me`
    Timer,
    /// `remind me in 10 min to stretch`
    Remind,
    /// `fire up steam`
    Open,
    /// `look up gpui docs`
    Search,
    /// `volume 30%`
    Volume,
}

/// Kind of detail inside a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotKind {
    /// `20m`, `half an hour`
    Duration,
    /// `5pm`, `tomorrow 7am`
    Time,
    /// free text, e.g. what to be reminded of
    Text,
    /// `30%`
    Percent,
    /// an application name
    App,
    /// a search query
    Query,
    /// a follow-up action, e.g. `then notify me`
    Action,
}

impl Intent {
    /// Every intent, in a fixed order (model rows follow this order).
    pub const ALL: &'static [Intent] = &[
        Intent::Timer,
        Intent::Remind,
        Intent::Open,
        Intent::Search,
        Intent::Volume,
    ];

    /// Stable lowercase name (used in training data and the model file).
    pub fn name(self) -> &'static str {
        match self {
            Intent::Timer => "timer",
            Intent::Remind => "remind",
            Intent::Open => "open",
            Intent::Search => "search",
            Intent::Volume => "volume",
        }
    }

    /// Slots this intent may contain; anything else is never predicted.
    pub fn slots(self) -> &'static [SlotKind] {
        use SlotKind::*;
        match self {
            Intent::Timer => &[Duration, Action],
            Intent::Remind => &[Duration, Time, Text],
            Intent::Open => &[App],
            Intent::Search => &[Query],
            Intent::Volume => &[Percent],
        }
    }
}

impl SlotKind {
    /// Every slot kind, in a fixed order (model rows follow this order).
    pub const ALL: &'static [SlotKind] = &[
        SlotKind::Duration,
        SlotKind::Time,
        SlotKind::Text,
        SlotKind::Percent,
        SlotKind::App,
        SlotKind::Query,
        SlotKind::Action,
    ];

    /// Stable uppercase name, e.g. `DUR` (used in training data).
    pub fn name(self) -> &'static str {
        match self {
            SlotKind::Duration => "DUR",
            SlotKind::Time => "TIME",
            SlotKind::Text => "TEXT",
            SlotKind::Percent => "PCT",
            SlotKind::App => "APP",
            SlotKind::Query => "QUERY",
            SlotKind::Action => "ACTION",
        }
    }
}
