mod features;
mod listeners;
mod state;

pub use features::FeatureKind;
#[cfg(feature = "clipboard")]
pub use features::clipboard::{Clipboard, ClipboardChange, ClipboardHandle};
#[cfg(feature = "keyboard")]
pub use features::keyboard::{Keyboard, Layout, LayoutHandle};
#[cfg(feature = "outputs")]
pub use features::outputs::{Output, Outputs, OutputsHandle};
pub use listeners::Subscription;
use state::WaylandState;
use wayland_client::{Connection, EventQueue};

/// Single Wayland listener with pluggable features.
///
/// ```ignore
/// let mut wl = WaylandWatcher::new()?;
/// let clipboard = wl.add(Clipboard::default());
/// let layout = wl.add(Keyboard::default());
/// wl.spawn();
///
/// // anywhere, any time; lives as long as `_sub`
/// let _sub = clipboard.subscribe(move |c| { tx.send(c).ok(); });
/// ```
///
/// Note: this is a separate connection without surfaces, so it never gets
/// keyboard focus. Keymaps arrive regardless; layout-group switches
/// (`Modifiers.group`) only if the compositor sends them unfocused.
pub struct WaylandWatcher {
    conn: Connection,
    state: WaylandState,
}

impl WaylandWatcher {
    pub fn new() -> Option<Self> {
        let conn = Connection::connect_to_env().ok()?;
        Some(Self {
            conn,
            state: WaylandState::default(),
        })
    }

    pub fn add<F: FeatureKind>(&mut self, f: F) -> F::Handle {
        let h = f.handle(&self.conn);
        self.state.features.push(f.into());
        h
    }

    pub fn spawn(self) {
        let Self { conn, mut state } = self;
        if state.features.is_empty() {
            return;
        }
        let Some(queue) = Self::handshake(&conn, &mut state) else {
            return;
        };
        Self::listen(queue, state);
    }

    /// Initial roundtrips on the calling thread, so handles hold data on return.
    /// 1: globals → binds   2: seat caps, output info   3: keymap, selection
    fn handshake(conn: &Connection, state: &mut WaylandState) -> Option<EventQueue<WaylandState>> {
        let mut queue = conn.new_event_queue();
        conn.display().get_registry(&queue.handle(), ());
        for _ in 0..3 {
            queue.roundtrip(state).ok()?;
        }
        Some(queue)
    }

    /// Keeps dispatching on a background thread until the connection closes.
    fn listen(mut queue: EventQueue<WaylandState>, mut state: WaylandState) {
        let _ = std::thread::Builder::new()
            .name("wayland".into())
            .spawn(move || {
                while queue.blocking_dispatch(&mut state).is_ok() {}
                eprintln!("wayland: connection closed");
            });
    }
}
