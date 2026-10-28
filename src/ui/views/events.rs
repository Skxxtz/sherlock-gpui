/// Whether the shell still runs its default after the view saw an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Propagation {
    /// The view handled it; skip the default.
    Stop,
    /// Run the default.
    #[default]
    Continue,
}
