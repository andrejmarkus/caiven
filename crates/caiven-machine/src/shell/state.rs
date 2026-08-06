//! The shell's state machine: which screen is up, where the cursor is, and
//! what every button press does from there.
//!
//! Deliberately free of SDL, of the raster surface and of the filesystem, so
//! the whole navigation graph is exhaustively unit-testable. Anything the
//! shell cannot do by itself — load a cart, delete a file, save a state — is
//! returned as an [`Effect`] for the caller to carry out.
//!
//! The graph follows the design handoff's behavioral prototype, with two
//! deliberate departures where the prototype leaks state. Both are noted at
//! the field that fixes them.

use std::time::Duration;

use crate::shell::settings::{Pane, Row, RowKind, SettingId, Settings};
use crate::shell::theme::motion;

/// How long the boot screen holds before it hands over to the library.
pub const BOOT_DURATION: Duration = motion::BOOT;

/// The shell's screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Boot,
    Library,
    Detail,
    Loading,
    Playing,
    Pause,
    Settings,
    Controls,
    Port,
    Crash,
}

impl Screen {
    /// Whether the status and legend bars are drawn on this screen.
    ///
    /// The immersive screens own the full panel: boot and loading are their
    /// own compositions, playing belongs to the cart, and pause and crash sit
    /// over a frozen frame.
    pub fn has_chrome(self) -> bool {
        matches!(
            self,
            Screen::Library | Screen::Detail | Screen::Settings | Screen::Controls | Screen::Port
        )
    }
}

/// The six console buttons plus the two menu keys.
///
/// START and SELECT are how the handoff's legends reach Settings and Port.
/// Whether a strictly six-button device can produce them is still open (see
/// the `?` in SPEC §C); it is a mapping question in `platform/input.rs`, not
/// a shape question here — this graph is the same either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellButton {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
}

/// Which action the cart detail screen has focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailAction {
    #[default]
    Play,
    Delete,
}

/// How the Port screen orders its listing. SELECT cycles through these
/// (search/tag/author filtering is out of scope this milestone — SPEC §C —
/// so a sort chip is the only query control the shell exposes).
///
/// Defined here rather than in `port_client` so `state.rs` stays free of
/// network concerns (per this module's doc comment) while `port_client`
/// (the host-side HTTP client) depends on this type, not the reverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PortSort {
    #[default]
    New,
    Popular,
    Trending,
    Top,
}

impl PortSort {
    /// The next sort in the cycle SELECT steps through.
    pub fn next(self) -> Self {
        match self {
            PortSort::New => PortSort::Popular,
            PortSort::Popular => PortSort::Trending,
            PortSort::Trending => PortSort::Top,
            PortSort::Top => PortSort::New,
        }
    }

    /// The `?sort=` value the Port API's `Sort::parse` recognizes
    /// (`caiven-port/src/db.rs`).
    pub fn query_value(self) -> &'static str {
        match self {
            PortSort::New => "new",
            PortSort::Popular => "popular",
            PortSort::Trending => "trending",
            PortSort::Top => "top",
        }
    }

    /// What the legend bar and the screen's own sort chip show.
    pub fn legend_label(self) -> &'static str {
        match self {
            PortSort::New => "Sort: New",
            PortSort::Popular => "Sort: Popular",
            PortSort::Trending => "Sort: Trending",
            PortSort::Top => "Sort: Top",
        }
    }
}

/// Which of the settings screen's two columns has the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Column {
    /// The pane list down the left edge.
    #[default]
    Rail,
    /// The rows of the selected pane.
    Rows,
}

/// A row of the pause overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseItem {
    Resume,
    SaveState,
    LoadState,
    ResetCart,
    Settings,
    Quit,
}

impl PauseItem {
    pub const ALL: [PauseItem; 6] = [
        PauseItem::Resume,
        PauseItem::SaveState,
        PauseItem::LoadState,
        PauseItem::ResetCart,
        PauseItem::Settings,
        PauseItem::Quit,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PauseItem::Resume => "Resume",
            PauseItem::SaveState => "Save state",
            PauseItem::LoadState => "Load state",
            PauseItem::ResetCart => "Reset cart",
            PauseItem::Settings => "Settings",
            PauseItem::Quit => "Quit to library",
        }
    }

    /// Whether the row is styled as destructive — it discards the session.
    pub fn is_destructive(self) -> bool {
        matches!(self, PauseItem::Quit)
    }
}

/// The six bindable buttons, in the order the remap screen lists them.
pub const BIND_ORDER: [ShellButton; 6] = [
    ShellButton::Up,
    ShellButton::Down,
    ShellButton::Left,
    ShellButton::Right,
    ShellButton::A,
    ShellButton::B,
];

/// Something the shell needs the host to do.
///
/// The state machine has already moved itself; an effect is the part it
/// cannot perform, not a request for permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Begin loading the library cart at this index. The shell is already on
    /// the loading screen; the host answers with [`ShellState::cart_ready`]
    /// or [`ShellState::cart_failed`].
    LoadCart(usize),
    /// Abandon the load in flight.
    CancelLoad,
    /// Delete the library cart at this index. Already removed from the count.
    DeleteCart(usize),
    /// Restart the running cart from its first frame.
    ResetCart,
    /// Tear the VM down and go back to the library.
    QuitToLibrary,
    SaveState,
    LoadState,
    /// Begin downloading the Port listing at this index.
    StartDownload(usize),
    /// (Re)fetch the Port listing for the current sort — entering the Port
    /// screen, or SELECT cycling the sort.
    RefreshPort,
    /// A setting changed; persist it.
    SettingsChanged,
    /// The remap screen wants the next physical input captured.
    ListenForBind(usize),
    /// Exit the process entirely, back to the device's own launcher.
    QuitApp,
}

/// One legend entry: a button chip and what it does here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Legend {
    /// The chip's text: `A`, `B`, `START`, `SELECT` or `◄►`.
    pub chip: &'static str,
    pub label: &'static str,
    /// Drawn as the filled ember chip rather than the outlined one.
    pub primary: bool,
    /// Pushed to the far end of the bar.
    pub trailing: bool,
}

const fn legend(chip: &'static str, label: &'static str) -> Legend {
    Legend {
        chip,
        label,
        primary: false,
        trailing: false,
    }
}

const fn primary(chip: &'static str, label: &'static str) -> Legend {
    Legend {
        chip,
        label,
        primary: true,
        trailing: false,
    }
}

const fn trailing(chip: &'static str, label: &'static str) -> Legend {
    Legend {
        chip,
        label,
        primary: false,
        trailing: true,
    }
}

/// Why the crash screen is up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashInfo {
    /// The real error text — an `mlua` message, or the context string from a
    /// failed load. Never a synthesized stand-in.
    pub message: String,
    /// The frame the cart died on, if it had started running.
    pub frame: Option<u64>,
}

/// The whole shell state.
#[derive(Debug, Clone)]
pub struct ShellState {
    screen: Screen,
    boot_elapsed: Duration,

    /// How many carts the library holds. The list itself is loaded elsewhere;
    /// navigation only needs the count and the index it lands on.
    cart_count: usize,
    /// The library cursor. `cart_count` is the trailing Port tile, so the
    /// valid range is `0..=cart_count` and an empty library sits on the tile.
    sel: usize,
    detail_action: DetailAction,

    pause_item: usize,

    pane: Pane,
    column: Column,
    row: usize,
    /// Where B leaves the settings screen for.
    ///
    /// The prototype always returns to the library, which silently drops a
    /// running cart when you opened Settings from the pause menu. Tracking
    /// the origin fixes that.
    settings_return: Screen,

    bind_index: usize,
    listening: bool,
    /// The display label for each of `BIND_ORDER`'s six buttons — what the
    /// remap screen shows. The host seeds this from `controls.toml` and
    /// updates it as captures land; the shell never reads the file itself.
    binds: [String; 6],

    port_count: usize,
    port_index: usize,
    port_sort: PortSort,
    downloading: Option<usize>,
    /// Where B leaves the Port screen for — same reasoning as
    /// `settings_return`.
    port_return: Screen,

    settings: Settings,
    crash: Option<CrashInfo>,
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellState {
    pub fn new() -> Self {
        Self {
            screen: Screen::Boot,
            boot_elapsed: Duration::ZERO,
            cart_count: 0,
            sel: 0,
            detail_action: DetailAction::default(),
            pause_item: 0,
            pane: Pane::Video,
            column: Column::default(),
            row: 0,
            settings_return: Screen::Library,
            bind_index: 0,
            listening: false,
            binds: Default::default(),
            port_count: 0,
            port_index: 0,
            port_sort: PortSort::default(),
            downloading: None,
            port_return: Screen::Library,
            settings: Settings::default(),
            crash: None,
        }
    }

    // --- readers -------------------------------------------------------

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn cart_count(&self) -> usize {
        self.cart_count
    }

    pub fn selected(&self) -> usize {
        self.sel
    }

    /// The selected library cart, or `None` on the Port tile / empty library.
    pub fn selected_cart(&self) -> Option<usize> {
        (self.sel < self.cart_count).then_some(self.sel)
    }

    /// Whether the cursor sits on the trailing "browse the Port" tile.
    pub fn on_port_tile(&self) -> bool {
        self.sel >= self.cart_count
    }

    pub fn detail_action(&self) -> DetailAction {
        self.detail_action
    }

    pub fn pause_item(&self) -> PauseItem {
        PauseItem::ALL[self.pause_item.min(PauseItem::ALL.len() - 1)]
    }

    pub fn pane(&self) -> Pane {
        self.pane
    }

    pub fn column(&self) -> Column {
        self.column
    }

    /// The focused settings row, or `None` while the cursor is on the rail.
    pub fn settings_row(&self) -> Option<&'static Row> {
        if self.column == Column::Rail {
            return None;
        }
        self.current_row()
    }

    pub fn bind_index(&self) -> usize {
        self.bind_index
    }

    /// Whether the remap screen is waiting for a physical input.
    pub fn is_listening(&self) -> bool {
        self.listening
    }

    /// The display label for each of `BIND_ORDER`'s six buttons, in order.
    pub fn binds(&self) -> &[String; 6] {
        &self.binds
    }

    pub fn port_index(&self) -> usize {
        self.port_index
    }

    pub fn port_sort(&self) -> PortSort {
        self.port_sort
    }

    pub fn downloading(&self) -> Option<usize> {
        self.downloading
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn crash(&self) -> Option<&CrashInfo> {
        self.crash.as_ref()
    }

    pub fn boot_elapsed(&self) -> Duration {
        self.boot_elapsed
    }

    // --- host notifications --------------------------------------------

    /// Seeds the starting settings (e.g. `--scale`/`--aspect` CLI flags)
    /// before the first frame. Once the shell is running, settings only
    /// ever change through [`Self::press`] on the Settings screen.
    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    /// Seeds the remap screen's labels from `controls.toml` before the
    /// first frame, same convention as [`Self::set_settings`].
    pub fn set_binds(&mut self, binds: [String; 6]) {
        self.binds = binds;
    }

    /// Replaces the library size, keeping the cursor in range.
    pub fn set_cart_count(&mut self, count: usize) {
        self.cart_count = count;
        self.sel = self.sel.min(count);
    }

    /// Replaces the Port listing size, keeping the cursor in range.
    pub fn set_port_count(&mut self, count: usize) {
        self.port_count = count;
        self.port_index = self.port_index.min(count.saturating_sub(1));
    }

    /// The cart finished loading and the VM is running it.
    pub fn cart_ready(&mut self) {
        if self.screen == Screen::Loading {
            self.screen = Screen::Playing;
        }
    }

    /// The cart failed to load, or died while running.
    pub fn cart_failed(&mut self, message: impl Into<String>, frame: Option<u64>) {
        self.crash = Some(CrashInfo {
            message: message.into(),
            frame,
        });
        self.screen = Screen::Crash;
    }

    /// A download finished; the cart joined the library.
    pub fn download_finished(&mut self) {
        self.downloading = None;
        self.cart_count += 1;
    }

    /// A download failed or was abandoned.
    pub fn download_failed(&mut self) {
        self.downloading = None;
    }

    /// The remap screen captured a new binding for the focused button,
    /// already written to `controls.toml` by the host — `label` is what the
    /// screen now shows for it.
    pub fn bind_captured(&mut self, label: impl Into<String>) {
        self.listening = false;
        self.binds[self.bind_index] = label.into();
    }

    /// Advances wall-clock-driven transitions. `dt` is real elapsed time, so
    /// a stalled frame loop shortens the boot screen rather than stretching
    /// it (SPEC V35).
    pub fn tick(&mut self, dt: Duration) {
        if self.screen != Screen::Boot {
            return;
        }
        self.boot_elapsed = self.boot_elapsed.saturating_add(dt);
        if self.boot_elapsed >= BOOT_DURATION {
            self.screen = Screen::Library;
        }
    }

    // --- input ---------------------------------------------------------

    /// Handles one button press, returning what the host must do about it.
    ///
    /// While the remap screen is listening, presses belong to the capture,
    /// not to navigation — the caller routes the raw input to
    /// [`ShellState::bind_captured`] instead.
    pub fn press(&mut self, button: ShellButton) -> Option<Effect> {
        if self.listening {
            return None;
        }
        match self.screen {
            Screen::Boot => self.press_boot(button),
            Screen::Library => self.press_library(button),
            Screen::Detail => self.press_detail(button),
            Screen::Loading => self.press_loading(button),
            Screen::Playing => self.press_playing(button),
            Screen::Pause => self.press_pause(button),
            Screen::Settings => self.press_settings(button),
            Screen::Controls => self.press_controls(button),
            Screen::Port => self.press_port(button),
            Screen::Crash => self.press_crash(button),
        }
    }

    fn press_boot(&mut self, button: ShellButton) -> Option<Effect> {
        if matches!(button, ShellButton::A | ShellButton::B | ShellButton::Start) {
            self.boot_elapsed = BOOT_DURATION;
            self.screen = Screen::Library;
        }
        None
    }

    fn press_library(&mut self, button: ShellButton) -> Option<Effect> {
        match button {
            ShellButton::Left => {
                self.sel = self.sel.saturating_sub(1);
                None
            }
            ShellButton::Right => {
                self.sel = (self.sel + 1).min(self.cart_count);
                None
            }
            ShellButton::A => match self.selected_cart() {
                Some(index) => self.begin_load(index),
                None => Some(self.open_port(Screen::Library)),
            },
            ShellButton::B => {
                if self.selected_cart().is_some() {
                    self.detail_action = DetailAction::Play;
                    self.screen = Screen::Detail;
                }
                None
            }
            ShellButton::Start => {
                self.open_settings(Screen::Library);
                None
            }
            ShellButton::Select => Some(self.open_port(Screen::Library)),
            ShellButton::Up | ShellButton::Down => None,
        }
    }

    fn press_detail(&mut self, button: ShellButton) -> Option<Effect> {
        match button {
            ShellButton::Up | ShellButton::Left => {
                self.detail_action = DetailAction::Play;
                None
            }
            ShellButton::Down | ShellButton::Right => {
                self.detail_action = DetailAction::Delete;
                None
            }
            ShellButton::A => {
                let index = self.selected_cart()?;
                match self.detail_action {
                    DetailAction::Play => self.begin_load(index),
                    DetailAction::Delete => Some(self.delete_selected(index)),
                }
            }
            ShellButton::B => {
                self.screen = Screen::Library;
                None
            }
            ShellButton::Start => {
                self.open_settings(Screen::Library);
                None
            }
            ShellButton::Select => None,
        }
    }

    fn press_loading(&mut self, button: ShellButton) -> Option<Effect> {
        if button == ShellButton::B {
            self.screen = Screen::Library;
            return Some(Effect::CancelLoad);
        }
        None
    }

    fn press_playing(&mut self, button: ShellButton) -> Option<Effect> {
        // Every other button belongs to the cart (SPEC V37).
        if button == ShellButton::Start {
            self.pause_item = 0;
            self.screen = Screen::Pause;
        }
        None
    }

    fn press_pause(&mut self, button: ShellButton) -> Option<Effect> {
        let len = PauseItem::ALL.len();
        match button {
            ShellButton::Up => {
                self.pause_item = (self.pause_item + len - 1) % len;
                None
            }
            ShellButton::Down => {
                self.pause_item = (self.pause_item + 1) % len;
                None
            }
            ShellButton::A => match self.pause_item() {
                PauseItem::Resume => {
                    self.screen = Screen::Playing;
                    None
                }
                PauseItem::SaveState => Some(Effect::SaveState),
                PauseItem::LoadState => Some(Effect::LoadState),
                PauseItem::ResetCart => {
                    self.screen = Screen::Playing;
                    Some(Effect::ResetCart)
                }
                PauseItem::Settings => {
                    self.open_settings(Screen::Pause);
                    None
                }
                PauseItem::Quit => {
                    self.screen = Screen::Library;
                    Some(Effect::QuitToLibrary)
                }
            },
            ShellButton::B | ShellButton::Start => {
                self.screen = Screen::Playing;
                None
            }
            ShellButton::Left | ShellButton::Right | ShellButton::Select => None,
        }
    }

    fn press_settings(&mut self, button: ShellButton) -> Option<Effect> {
        match self.column {
            Column::Rail => self.press_settings_rail(button),
            Column::Rows => self.press_settings_rows(button),
        }
    }

    fn press_settings_rail(&mut self, button: ShellButton) -> Option<Effect> {
        match button {
            ShellButton::Up => {
                self.pane = self.pane.stepped(-1);
                self.row = 0;
                None
            }
            ShellButton::Down => {
                self.pane = self.pane.stepped(1);
                self.row = 0;
                None
            }
            ShellButton::Right | ShellButton::A => {
                self.column = Column::Rows;
                self.row = 0;
                None
            }
            ShellButton::B | ShellButton::Start => {
                self.screen = self.settings_return;
                None
            }
            ShellButton::Left | ShellButton::Select => None,
        }
    }

    fn press_settings_rows(&mut self, button: ShellButton) -> Option<Effect> {
        let rows = self.pane.rows();
        if rows.is_empty() {
            self.column = Column::Rail;
            return None;
        }
        self.row = self.row.min(rows.len() - 1);
        let row = rows[self.row];
        match button {
            ShellButton::Up => {
                self.row = (self.row + rows.len() - 1) % rows.len();
                None
            }
            ShellButton::Down => {
                self.row = (self.row + 1) % rows.len();
                None
            }
            ShellButton::Left => {
                if row.kind.is_adjustable() {
                    self.adjust(row.id, -1)
                } else {
                    self.column = Column::Rail;
                    None
                }
            }
            ShellButton::Right => {
                if row.kind.is_adjustable() {
                    self.adjust(row.id, 1)
                } else {
                    None
                }
            }
            ShellButton::A => {
                if row.kind == RowKind::Action {
                    self.run_action(row.id)
                } else {
                    self.adjust(row.id, 1)
                }
            }
            ShellButton::B => {
                self.column = Column::Rail;
                None
            }
            ShellButton::Start | ShellButton::Select => None,
        }
    }

    fn press_controls(&mut self, button: ShellButton) -> Option<Effect> {
        let len = BIND_ORDER.len();
        match button {
            ShellButton::Up => {
                self.bind_index = (self.bind_index + len - 1) % len;
                None
            }
            ShellButton::Down => {
                self.bind_index = (self.bind_index + 1) % len;
                None
            }
            ShellButton::A => {
                self.listening = true;
                Some(Effect::ListenForBind(self.bind_index))
            }
            ShellButton::B => {
                self.screen = Screen::Settings;
                self.pane = Pane::Controls;
                self.column = Column::Rows;
                self.row = 0;
                None
            }
            ShellButton::Left | ShellButton::Right | ShellButton::Start | ShellButton::Select => {
                None
            }
        }
    }

    fn press_port(&mut self, button: ShellButton) -> Option<Effect> {
        match button {
            ShellButton::Up => {
                if self.port_count > 0 {
                    self.port_index = (self.port_index + self.port_count - 1) % self.port_count;
                }
                None
            }
            ShellButton::Down => {
                if self.port_count > 0 {
                    self.port_index = (self.port_index + 1) % self.port_count;
                }
                None
            }
            ShellButton::A => {
                if self.downloading.is_some() || self.port_index >= self.port_count {
                    return None;
                }
                self.downloading = Some(self.port_index);
                Some(Effect::StartDownload(self.port_index))
            }
            ShellButton::B => {
                self.screen = self.port_return;
                None
            }
            ShellButton::Select => {
                self.port_sort = self.port_sort.next();
                self.port_index = 0;
                Some(Effect::RefreshPort)
            }
            ShellButton::Left | ShellButton::Right | ShellButton::Start => None,
        }
    }

    fn press_crash(&mut self, button: ShellButton) -> Option<Effect> {
        match button {
            ShellButton::A => {
                let index = self.selected_cart()?;
                self.crash = None;
                self.begin_load(index)
            }
            ShellButton::B => {
                self.crash = None;
                self.screen = Screen::Library;
                None
            }
            _ => None,
        }
    }

    // --- shared transitions --------------------------------------------

    fn begin_load(&mut self, index: usize) -> Option<Effect> {
        self.screen = Screen::Loading;
        Some(Effect::LoadCart(index))
    }

    fn delete_selected(&mut self, index: usize) -> Effect {
        self.cart_count -= 1;
        self.sel = self.sel.min(self.cart_count);
        self.detail_action = DetailAction::Play;
        self.screen = Screen::Library;
        Effect::DeleteCart(index)
    }

    fn open_settings(&mut self, from: Screen) {
        self.settings_return = from;
        self.screen = Screen::Settings;
        self.column = Column::Rail;
        self.pane = Pane::Video;
        self.row = 0;
    }

    fn open_port(&mut self, from: Screen) -> Effect {
        self.port_return = from;
        self.screen = Screen::Port;
        self.port_index = 0;
        Effect::RefreshPort
    }

    fn current_row(&self) -> Option<&'static Row> {
        let rows = self.pane.rows();
        rows.get(self.row.min(rows.len().saturating_sub(1)))
    }

    fn adjust(&mut self, id: SettingId, dir: i32) -> Option<Effect> {
        self.settings
            .adjust(id, dir)
            .then_some(Effect::SettingsChanged)
    }

    fn run_action(&mut self, id: SettingId) -> Option<Effect> {
        match id {
            SettingId::Rebind => {
                self.screen = Screen::Controls;
                self.bind_index = 0;
                self.listening = false;
                None
            }
            SettingId::Browse => Some(self.open_port(Screen::Settings)),
            SettingId::RestoreDefaults => {
                let defaults = Settings::default();
                if self.settings == defaults {
                    return None;
                }
                self.settings = defaults;
                Some(Effect::SettingsChanged)
            }
            SettingId::QuitConsole => Some(Effect::QuitApp),
            _ => None,
        }
    }

    // --- legend --------------------------------------------------------

    /// What the legend bar shows on the current screen.
    ///
    /// Empty on the screens without chrome — they draw their own hints, if
    /// any.
    pub fn legend(&self) -> Vec<Legend> {
        match self.screen {
            Screen::Library => {
                if self.selected_cart().is_some() {
                    vec![
                        primary("A", "Play"),
                        legend("B", "Details"),
                        trailing("START", "Settings"),
                    ]
                } else {
                    vec![
                        primary("A", "Browse the Port"),
                        trailing("START", "Settings"),
                    ]
                }
            }
            Screen::Detail => vec![
                primary("A", "Select"),
                legend("B", "Back"),
                legend("◄►", "Switch action"),
            ],
            Screen::Settings => match self.column {
                Column::Rail => vec![primary("A", "Open section"), legend("B", "Back")],
                Column::Rows => vec![
                    primary("A", "Change"),
                    legend("B", "Sections"),
                    legend("◄►", "Adjust"),
                ],
            },
            Screen::Controls => {
                if self.listening {
                    Vec::new()
                } else {
                    vec![primary("A", "Rebind"), legend("B", "Back")]
                }
            }
            Screen::Port => {
                let back = if self.port_return == Screen::Settings {
                    "Settings"
                } else {
                    "Library"
                };
                vec![
                    primary("A", "Download"),
                    legend("B", back),
                    trailing("SELECT", self.port_sort.legend_label()),
                ]
            }
            Screen::Pause => vec![primary("A", "Select"), legend("B", "Resume")],
            Screen::Boot | Screen::Loading | Screen::Playing | Screen::Crash => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::scaling::ScaleMode;

    /// A shell past the boot screen with `carts` carts in the library.
    fn library(carts: usize) -> ShellState {
        let mut state = ShellState::new();
        state.set_cart_count(carts);
        state.tick(BOOT_DURATION);
        assert_eq!(state.screen(), Screen::Library);
        state
    }

    fn press_all(state: &mut ShellState, buttons: &[ShellButton]) {
        for button in buttons {
            state.press(*button);
        }
    }

    #[test]
    fn boot_hands_over_on_real_elapsed_time() {
        let mut state = ShellState::new();
        state.tick(Duration::from_millis(400));
        assert_eq!(state.screen(), Screen::Boot);
        state.tick(Duration::from_millis(400));
        assert_eq!(state.screen(), Screen::Boot);
        state.tick(Duration::from_millis(400));
        assert_eq!(state.screen(), Screen::Library);
    }

    #[test]
    fn boot_can_be_skipped_and_never_replays() {
        let mut state = ShellState::new();
        assert_eq!(state.press(ShellButton::A), None);
        assert_eq!(state.screen(), Screen::Library);
        state.tick(Duration::from_secs(5));
        assert_eq!(state.screen(), Screen::Library);
    }

    #[test]
    fn an_empty_library_sits_on_the_port_tile() {
        let mut state = library(0);
        assert!(state.on_port_tile());
        assert_eq!(state.selected_cart(), None);
        assert_eq!(state.press(ShellButton::A), Some(Effect::RefreshPort));
        assert_eq!(state.screen(), Screen::Port);
    }

    #[test]
    fn library_cursor_clamps_at_both_ends_with_the_port_tile_last() {
        let mut state = library(2);
        assert_eq!(state.selected(), 0);
        press_all(&mut state, &[ShellButton::Left, ShellButton::Left]);
        assert_eq!(state.selected(), 0);
        press_all(
            &mut state,
            &[ShellButton::Right, ShellButton::Right, ShellButton::Right],
        );
        // Two carts plus the trailing tile: the last index is 2, not 3.
        assert_eq!(state.selected(), 2);
        assert!(state.on_port_tile());
    }

    #[test]
    fn a_on_a_cart_loads_it_and_b_opens_its_detail() {
        let mut state = library(3);
        state.press(ShellButton::Right);
        assert_eq!(state.press(ShellButton::A), Some(Effect::LoadCart(1)));
        assert_eq!(state.screen(), Screen::Loading);

        state.cart_ready();
        assert_eq!(state.screen(), Screen::Playing);

        let mut state = library(3);
        state.press(ShellButton::B);
        assert_eq!(state.screen(), Screen::Detail);
        assert_eq!(state.detail_action(), DetailAction::Play);
    }

    #[test]
    fn b_on_the_port_tile_does_not_open_a_detail_screen() {
        let mut state = library(1);
        state.press(ShellButton::Right);
        assert!(state.on_port_tile());
        assert_eq!(state.press(ShellButton::B), None);
        assert_eq!(state.screen(), Screen::Library);
    }

    #[test]
    fn detail_delete_removes_the_cart_and_pulls_the_cursor_back() {
        let mut state = library(2);
        state.press(ShellButton::Right);
        state.press(ShellButton::B);
        assert_eq!(state.screen(), Screen::Detail);
        state.press(ShellButton::Down);
        assert_eq!(state.detail_action(), DetailAction::Delete);
        assert_eq!(state.press(ShellButton::A), Some(Effect::DeleteCart(1)));
        assert_eq!(state.screen(), Screen::Library);
        assert_eq!(state.cart_count(), 1);
        // One cart left, so index 1 is now the Port tile — still in range.
        assert_eq!(state.selected(), 1);
        assert!(state.on_port_tile());
    }

    #[test]
    fn loading_can_be_cancelled_back_to_the_library() {
        let mut state = library(1);
        state.press(ShellButton::A);
        assert_eq!(state.press(ShellButton::B), Some(Effect::CancelLoad));
        assert_eq!(state.screen(), Screen::Library);
    }

    #[test]
    fn a_failed_load_lands_on_the_crash_screen_with_the_real_message() {
        let mut state = library(1);
        state.press(ShellButton::A);
        state.cart_failed("attempt to index a nil value (global 'ply')", Some(412));
        assert_eq!(state.screen(), Screen::Crash);
        let crash = state.crash().expect("crash info");
        assert!(crash.message.contains("nil value"));
        assert_eq!(crash.frame, Some(412));
    }

    #[test]
    fn crash_retries_the_same_cart_or_leaves_to_the_library() {
        let mut state = library(2);
        state.press(ShellButton::Right);
        state.press(ShellButton::A);
        state.cart_failed("boom", None);
        assert_eq!(state.press(ShellButton::A), Some(Effect::LoadCart(1)));
        assert_eq!(state.screen(), Screen::Loading);
        assert!(state.crash().is_none());

        state.cart_failed("boom", None);
        assert_eq!(state.press(ShellButton::B), None);
        assert_eq!(state.screen(), Screen::Library);
        assert!(state.crash().is_none());
    }

    #[test]
    fn only_start_reaches_the_pause_menu_while_playing() {
        let mut state = library(1);
        state.press(ShellButton::A);
        state.cart_ready();
        for button in [
            ShellButton::Up,
            ShellButton::Down,
            ShellButton::Left,
            ShellButton::Right,
            ShellButton::A,
            ShellButton::B,
            ShellButton::Select,
        ] {
            assert_eq!(state.press(button), None);
            assert_eq!(state.screen(), Screen::Playing, "{button:?} left Playing");
        }
        state.press(ShellButton::Start);
        assert_eq!(state.screen(), Screen::Pause);
        assert_eq!(state.pause_item(), PauseItem::Resume);
    }

    #[test]
    fn pause_wraps_and_every_item_does_what_it_says() {
        let mut state = library(1);
        state.press(ShellButton::A);
        state.cart_ready();
        state.press(ShellButton::Start);

        state.press(ShellButton::Up);
        assert_eq!(state.pause_item(), PauseItem::Quit);
        state.press(ShellButton::Down);
        assert_eq!(state.pause_item(), PauseItem::Resume);

        assert_eq!(state.press(ShellButton::A), None);
        assert_eq!(state.screen(), Screen::Playing);

        state.press(ShellButton::Start);
        state.press(ShellButton::Down);
        assert_eq!(state.press(ShellButton::A), Some(Effect::SaveState));
        assert_eq!(state.screen(), Screen::Pause, "saving stays in the menu");
        state.press(ShellButton::Down);
        assert_eq!(state.press(ShellButton::A), Some(Effect::LoadState));
        state.press(ShellButton::Down);
        assert_eq!(state.press(ShellButton::A), Some(Effect::ResetCart));
        assert_eq!(state.screen(), Screen::Playing);

        state.press(ShellButton::Start);
        for _ in 0..PauseItem::ALL.len() - 1 {
            state.press(ShellButton::Down);
        }
        assert_eq!(state.pause_item(), PauseItem::Quit);
        assert_eq!(state.press(ShellButton::A), Some(Effect::QuitToLibrary));
        assert_eq!(state.screen(), Screen::Library);
    }

    #[test]
    fn settings_opened_from_pause_returns_to_pause_not_the_library() {
        // The handoff prototype drops the running cart here; we do not.
        let mut state = library(1);
        state.press(ShellButton::A);
        state.cart_ready();
        state.press(ShellButton::Start);
        press_all(
            &mut state,
            &[
                ShellButton::Down,
                ShellButton::Down,
                ShellButton::Down,
                ShellButton::Down,
            ],
        );
        assert_eq!(state.pause_item(), PauseItem::Settings);
        state.press(ShellButton::A);
        assert_eq!(state.screen(), Screen::Settings);
        state.press(ShellButton::B);
        assert_eq!(state.screen(), Screen::Pause);
    }

    #[test]
    fn settings_rail_walks_panes_and_a_enters_the_rows() {
        let mut state = library(1);
        state.press(ShellButton::Start);
        assert_eq!(state.screen(), Screen::Settings);
        assert_eq!(state.pane(), Pane::Video);
        assert_eq!(state.column(), Column::Rail);
        assert!(state.settings_row().is_none());

        state.press(ShellButton::Up);
        assert_eq!(state.pane(), Pane::System, "rail wraps");
        state.press(ShellButton::Down);
        assert_eq!(state.pane(), Pane::Video);

        state.press(ShellButton::A);
        assert_eq!(state.column(), Column::Rows);
        assert_eq!(
            state.settings_row().map(|row| row.id),
            Some(SettingId::Scaling)
        );
        state.press(ShellButton::B);
        assert_eq!(state.column(), Column::Rail);
    }

    #[test]
    fn left_adjusts_an_adjustable_row_but_leaves_the_column_otherwise() {
        let mut state = library(1);
        press_all(&mut state, &[ShellButton::Start, ShellButton::A]);
        assert_eq!(
            state.press(ShellButton::Left),
            Some(Effect::SettingsChanged),
            "Scaling is a choice row, so Left cycles it"
        );
        assert_eq!(state.column(), Column::Rows);
        assert_eq!(state.settings().scaling, ScaleMode::Integer3x);

        // Port's first row is a readout: Left backs out instead.
        let mut state = library(1);
        state.press(ShellButton::Start);
        press_all(
            &mut state,
            &[ShellButton::Down, ShellButton::Down, ShellButton::Down],
        );
        assert_eq!(state.pane(), Pane::Port);
        state.press(ShellButton::A);
        assert_eq!(state.press(ShellButton::Left), None);
        assert_eq!(state.column(), Column::Rail);
    }

    #[test]
    fn restore_defaults_only_reports_a_change_when_something_changed() {
        let mut state = library(1);
        state.press(ShellButton::Start);
        press_all(&mut state, &[ShellButton::Up]); // System pane
        assert_eq!(state.pane(), Pane::System);
        state.press(ShellButton::A);
        press_all(&mut state, &[ShellButton::Down]); // Restore defaults
        assert_eq!(
            state.settings_row().map(|row| row.id),
            Some(SettingId::RestoreDefaults)
        );
        assert_eq!(state.press(ShellButton::A), None, "already at defaults");

        state.settings.show_fps = true;
        assert_eq!(state.press(ShellButton::A), Some(Effect::SettingsChanged));
        assert_eq!(*state.settings(), Settings::default());
    }

    #[test]
    fn rebind_action_opens_the_remap_screen_and_b_comes_back_to_its_pane() {
        let mut state = library(1);
        state.press(ShellButton::Start);
        press_all(&mut state, &[ShellButton::Down, ShellButton::Down]);
        assert_eq!(state.pane(), Pane::Controls);
        press_all(&mut state, &[ShellButton::A, ShellButton::A]);
        assert_eq!(state.screen(), Screen::Controls);
        assert_eq!(state.bind_index(), 0);

        state.press(ShellButton::B);
        assert_eq!(state.screen(), Screen::Settings);
        assert_eq!(state.pane(), Pane::Controls);
        assert_eq!(state.column(), Column::Rows);
    }

    #[test]
    fn listening_swallows_navigation_until_the_capture_resolves() {
        let mut state = library(1);
        state.press(ShellButton::Start);
        press_all(
            &mut state,
            &[
                ShellButton::Down,
                ShellButton::Down,
                ShellButton::A,
                ShellButton::A,
            ],
        );
        assert_eq!(state.screen(), Screen::Controls);
        state.press(ShellButton::Down);
        assert_eq!(state.bind_index(), 1);
        assert_eq!(
            state.press(ShellButton::A),
            Some(Effect::ListenForBind(1)),
            "A arms the capture for the focused button"
        );
        assert!(state.is_listening());

        // Nothing moves while armed, not even Back.
        assert_eq!(state.press(ShellButton::Down), None);
        assert_eq!(state.press(ShellButton::B), None);
        assert_eq!(state.bind_index(), 1);
        assert_eq!(state.screen(), Screen::Controls);

        state.bind_captured("KeyE");
        assert!(!state.is_listening());
        assert_eq!(state.binds()[1], "KeyE");
        state.press(ShellButton::Down);
        assert_eq!(state.bind_index(), 2);
    }

    #[test]
    fn bind_cursor_wraps_over_all_six_buttons() {
        let mut state = library(1);
        state.press(ShellButton::Start);
        press_all(
            &mut state,
            &[
                ShellButton::Down,
                ShellButton::Down,
                ShellButton::A,
                ShellButton::A,
            ],
        );
        state.press(ShellButton::Up);
        assert_eq!(state.bind_index(), BIND_ORDER.len() - 1);
        state.press(ShellButton::Down);
        assert_eq!(state.bind_index(), 0);
    }

    #[test]
    fn seeded_binds_show_until_a_capture_replaces_one() {
        let mut state = library(1);
        let seeded = [
            "ArrowUp, KeyW".to_string(),
            "ArrowDown, KeyS".to_string(),
            "ArrowLeft, KeyA".to_string(),
            "ArrowRight, KeyD".to_string(),
            "KeyJ".to_string(),
            "KeyK".to_string(),
        ];
        state.set_binds(seeded.clone());
        assert_eq!(state.binds(), &seeded);

        state.press(ShellButton::Start);
        press_all(
            &mut state,
            &[
                ShellButton::Down,
                ShellButton::Down,
                ShellButton::A,
                ShellButton::A,
            ],
        );
        assert_eq!(state.press(ShellButton::A), Some(Effect::ListenForBind(0)));
        state.bind_captured("KeyE");
        assert_eq!(state.binds()[0], "KeyE");
        // Every other row's label is untouched by an unrelated capture.
        assert_eq!(state.binds()[1], seeded[1]);
    }

    #[test]
    fn port_downloads_once_and_appends_to_the_library() {
        let mut state = library(1);
        state.set_port_count(3);
        state.press(ShellButton::Select);
        assert_eq!(state.screen(), Screen::Port);

        state.press(ShellButton::Up);
        assert_eq!(state.port_index(), 2, "the list wraps");
        state.press(ShellButton::Down);
        assert_eq!(state.port_index(), 0);

        assert_eq!(state.press(ShellButton::A), Some(Effect::StartDownload(0)));
        assert_eq!(state.downloading(), Some(0));
        assert_eq!(
            state.press(ShellButton::A),
            None,
            "a second press must not start a parallel download"
        );

        state.download_finished();
        assert_eq!(state.downloading(), None);
        assert_eq!(state.cart_count(), 2);
    }

    #[test]
    fn opening_port_requests_a_refresh() {
        let mut state = library(1);
        assert_eq!(state.press(ShellButton::Select), Some(Effect::RefreshPort));
        assert_eq!(state.screen(), Screen::Port);
    }

    #[test]
    fn select_cycles_sort_and_resets_the_cursor_and_requests_a_refresh() {
        let mut state = library(1);
        state.press(ShellButton::Select); // open Port
        state.set_port_count(3);
        state.press(ShellButton::Down);
        assert_eq!(state.port_index(), 1);

        assert_eq!(state.port_sort(), PortSort::New);
        assert_eq!(state.press(ShellButton::Select), Some(Effect::RefreshPort));
        assert_eq!(state.port_sort(), PortSort::Popular);
        assert_eq!(state.port_index(), 0, "changing sort resets the cursor");

        state.press(ShellButton::Select);
        state.press(ShellButton::Select);
        assert_eq!(state.port_sort(), PortSort::Top);
        state.press(ShellButton::Select);
        assert_eq!(state.port_sort(), PortSort::New, "the cycle wraps");
    }

    #[test]
    fn an_empty_port_listing_has_nothing_to_download() {
        let mut state = library(0);
        state.press(ShellButton::A);
        assert_eq!(state.screen(), Screen::Port);
        assert_eq!(state.press(ShellButton::A), None);
        assert_eq!(state.press(ShellButton::Down), None);
        assert_eq!(state.port_index(), 0);
    }

    #[test]
    fn port_opened_from_settings_returns_to_settings() {
        let mut state = library(1);
        state.press(ShellButton::Start);
        press_all(
            &mut state,
            &[ShellButton::Down, ShellButton::Down, ShellButton::Down],
        );
        assert_eq!(state.pane(), Pane::Port);
        press_all(&mut state, &[ShellButton::A, ShellButton::Down]);
        assert_eq!(
            state.settings_row().map(|row| row.id),
            Some(SettingId::Browse)
        );
        state.press(ShellButton::A);
        assert_eq!(state.screen(), Screen::Port);
        state.press(ShellButton::B);
        assert_eq!(state.screen(), Screen::Settings);
    }

    #[test]
    fn shrinking_the_library_keeps_the_cursor_in_range() {
        let mut state = library(5);
        press_all(&mut state, &[ShellButton::Right; 5]);
        assert_eq!(state.selected(), 5);
        state.set_cart_count(2);
        assert_eq!(state.selected(), 2);
        assert!(state.on_port_tile());
    }

    #[test]
    fn chrome_is_absent_exactly_on_the_immersive_screens() {
        for screen in [
            Screen::Boot,
            Screen::Loading,
            Screen::Playing,
            Screen::Pause,
            Screen::Crash,
        ] {
            assert!(!screen.has_chrome(), "{screen:?} must not draw chrome");
        }
        for screen in [
            Screen::Library,
            Screen::Detail,
            Screen::Settings,
            Screen::Controls,
            Screen::Port,
        ] {
            assert!(screen.has_chrome(), "{screen:?} must draw chrome");
        }
    }

    #[test]
    fn a_legend_appears_exactly_where_chrome_does_plus_pause() {
        let mut state = library(1);
        assert!(!state.legend().is_empty());
        state.press(ShellButton::A);
        assert!(state.legend().is_empty(), "loading has no legend");
        state.cart_ready();
        assert!(state.legend().is_empty(), "playing has no legend");
        state.press(ShellButton::Start);
        assert!(!state.legend().is_empty(), "pause draws its own");
    }

    #[test]
    fn the_library_legend_tracks_what_a_would_actually_do() {
        let mut state = library(1);
        assert_eq!(state.legend()[0].label, "Play");
        state.press(ShellButton::Right);
        assert_eq!(state.legend()[0].label, "Browse the Port");

        let state = library(0);
        assert_eq!(state.legend()[0].label, "Browse the Port");
    }

    #[test]
    fn every_legend_marks_exactly_one_primary_chip() {
        // The primary chip is the ember-filled one; two of them, or none,
        // would leave the player with no obvious default action.
        let mut state = library(2);
        state.set_port_count(2);
        let mut checked = 0;
        // Library → Detail → Library → Settings rail → Settings rows, then
        // out to the Port screen.
        let walk = [
            ShellButton::B,
            ShellButton::B,
            ShellButton::Start,
            ShellButton::A,
            ShellButton::B,
            ShellButton::B,
            ShellButton::Select,
        ];
        for button in walk {
            let entries = state.legend();
            if !entries.is_empty() {
                checked += 1;
                assert_eq!(
                    entries.iter().filter(|entry| entry.primary).count(),
                    1,
                    "{:?} legend",
                    state.screen()
                );
            }
            state.press(button);
        }
        assert!(checked >= walk.len(), "every step drew a legend");
    }
}
