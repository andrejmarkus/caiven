//! The Settings screen's content model: five panes of typed rows.
//!
//! The row's *kind* is what the navigation graph reads — it decides whether
//! Left/Right adjust a value or step back to the pane rail, and what A does.
//! So the table lives here, beside the values it edits, and `state.rs` walks
//! it without knowing what any individual setting means.
//!
//! Only settings the Machine can actually honor are listed. The design
//! handoff also mocks brightness, a scanline overlay, vibration and a sleep
//! timer; those are device features with no implementation behind them, and
//! a control that does nothing is worse than an absent one. They come back
//! when something can act on them.

use serde::{Deserialize, Serialize};

use crate::platform::scaling::{AspectMode, ScaleMode};

/// A settings section, in rail order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pane {
    Video,
    Audio,
    Controls,
    Port,
    System,
}

impl Pane {
    /// Every pane, in the order the rail lists them.
    pub const ALL: [Pane; 5] = [
        Pane::Video,
        Pane::Audio,
        Pane::Controls,
        Pane::Port,
        Pane::System,
    ];

    /// The rail label.
    pub fn label(self) -> &'static str {
        match self {
            Pane::Video => "Video",
            Pane::Audio => "Audio",
            Pane::Controls => "Controls",
            Pane::Port => "Port",
            Pane::System => "System",
        }
    }

    /// Position in `ALL`.
    pub fn index(self) -> usize {
        match self {
            Pane::Video => 0,
            Pane::Audio => 1,
            Pane::Controls => 2,
            Pane::Port => 3,
            Pane::System => 4,
        }
    }

    /// The pane `steps` positions away, wrapping in both directions.
    pub fn stepped(self, steps: isize) -> Pane {
        let len = Pane::ALL.len() as isize;
        let next = (self.index() as isize + steps).rem_euclid(len);
        Pane::ALL[next as usize]
    }

    /// The rows this pane shows, top to bottom.
    pub fn rows(self) -> &'static [Row] {
        match self {
            Pane::Video => VIDEO_ROWS,
            Pane::Audio => AUDIO_ROWS,
            Pane::Controls => CONTROLS_ROWS,
            Pane::Port => PORT_ROWS,
            Pane::System => SYSTEM_ROWS,
        }
    }
}

/// Identifies one row across every pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingId {
    Scaling,
    Aspect,
    ShowFps,
    MasterVolume,
    SfxVolume,
    MusicVolume,
    Rebind,
    Server,
    Browse,
    Version,
    RestoreDefaults,
    QuitConsole,
}

/// What a row does when you press Left, Right or A on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// Cycles through a fixed list of options.
    Choice { options: &'static [&'static str] },
    /// Flips between on and off.
    Toggle,
    /// A 0-100 value, stepped by `RANGE_STEP`.
    Range,
    /// Runs something; A is the only key it answers to.
    Action,
    /// Displays a runtime value. Not editable, so Left leaves the column.
    Readout,
}

impl RowKind {
    /// Whether Left/Right change the value rather than moving the cursor.
    pub fn is_adjustable(self) -> bool {
        matches!(
            self,
            RowKind::Choice { .. } | RowKind::Toggle | RowKind::Range
        )
    }
}

/// One line in a settings pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub id: SettingId,
    pub label: &'static str,
    /// The dim line under the label. Empty when the label says enough.
    pub sub: &'static str,
    pub kind: RowKind,
}

/// How far one Left/Right press moves a `Range` row.
pub const RANGE_STEP: i32 = 10;

/// Scaling options, in `ScaleMode` order rather than the mock's order — the
/// list reads better ascending, and `Settings` converts by value anyway.
pub const SCALING_OPTIONS: &[&str] = &["fit height", "integer 2×", "integer 3×"];
pub const ASPECT_OPTIONS: &[&str] = &["square", "stretch"];

const VIDEO_ROWS: &[Row] = &[
    Row {
        id: SettingId::Scaling,
        label: "Scaling",
        sub: "Integer keeps pixels square",
        kind: RowKind::Choice {
            options: SCALING_OPTIONS,
        },
    },
    Row {
        id: SettingId::Aspect,
        label: "Aspect",
        sub: "1:1 pixels, centered",
        kind: RowKind::Choice {
            options: ASPECT_OPTIONS,
        },
    },
    Row {
        id: SettingId::ShowFps,
        label: "Show fps",
        sub: "Top-right of the game frame",
        kind: RowKind::Toggle,
    },
];

const AUDIO_ROWS: &[Row] = &[
    Row {
        id: SettingId::MasterVolume,
        label: "Master volume",
        sub: "",
        kind: RowKind::Range,
    },
    Row {
        id: SettingId::SfxVolume,
        label: "Sound effects",
        sub: "The square/noise synth",
        kind: RowKind::Range,
    },
    Row {
        id: SettingId::MusicVolume,
        label: "Music",
        sub: "",
        kind: RowKind::Range,
    },
];

const CONTROLS_ROWS: &[Row] = &[Row {
    id: SettingId::Rebind,
    label: "Rebind buttons",
    sub: "Writes controls.toml",
    kind: RowKind::Action,
}];

const PORT_ROWS: &[Row] = &[
    Row {
        id: SettingId::Server,
        label: "Server",
        sub: "Where carts come from",
        kind: RowKind::Readout,
    },
    Row {
        id: SettingId::Browse,
        label: "Browse carts",
        sub: "",
        kind: RowKind::Action,
    },
];

const SYSTEM_ROWS: &[Row] = &[
    Row {
        id: SettingId::Version,
        label: "Version",
        sub: "",
        kind: RowKind::Readout,
    },
    Row {
        id: SettingId::RestoreDefaults,
        label: "Restore defaults",
        sub: "",
        kind: RowKind::Action,
    },
    Row {
        id: SettingId::QuitConsole,
        label: "Quit console",
        sub: "Exits back to the device's own menu",
        kind: RowKind::Action,
    },
];

/// The values the settings rows edit.
///
/// Persistence is not here — the shell writes this to TOML beside the binary
/// whenever `adjust` reports a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub scaling: ScaleMode,
    pub aspect: AspectMode,
    pub show_fps: bool,
    pub master_volume: u8,
    pub sfx_volume: u8,
    pub music_volume: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scaling: ScaleMode::default(),
            aspect: AspectMode::default(),
            show_fps: false,
            master_volume: 70,
            sfx_volume: 80,
            music_volume: 60,
        }
    }
}

impl Settings {
    /// Moves a row's value by `dir` (-1 or +1). Choices wrap; ranges clamp.
    ///
    /// Returns whether anything changed, so the caller only writes the file
    /// when it has to — the same press repeated at a range's ceiling should
    /// not keep re-saving.
    pub fn adjust(&mut self, id: SettingId, dir: i32) -> bool {
        let before = *self;
        match id {
            SettingId::Scaling => {
                let modes = [ScaleMode::Fit, ScaleMode::Integer2x, ScaleMode::Integer3x];
                self.scaling = modes[wrap(index_of(&modes, self.scaling), modes.len(), dir)];
            }
            SettingId::Aspect => {
                let modes = [AspectMode::Square, AspectMode::Stretch];
                self.aspect = modes[wrap(index_of(&modes, self.aspect), modes.len(), dir)];
            }
            SettingId::ShowFps => self.show_fps = !self.show_fps,
            SettingId::MasterVolume => self.master_volume = step(self.master_volume, dir),
            SettingId::SfxVolume => self.sfx_volume = step(self.sfx_volume, dir),
            SettingId::MusicVolume => self.music_volume = step(self.music_volume, dir),
            // Actions and readouts have no value to move.
            SettingId::Rebind
            | SettingId::Server
            | SettingId::Browse
            | SettingId::Version
            | SettingId::RestoreDefaults
            | SettingId::QuitConsole => {}
        }
        *self != before
    }

    /// The option index a `Choice` row should show as selected.
    pub fn choice_index(&self, id: SettingId) -> Option<usize> {
        match id {
            SettingId::Scaling => Some(match self.scaling {
                ScaleMode::Fit => 0,
                ScaleMode::Integer2x => 1,
                ScaleMode::Integer3x => 2,
            }),
            SettingId::Aspect => Some(match self.aspect {
                AspectMode::Square => 0,
                AspectMode::Stretch => 1,
            }),
            _ => None,
        }
    }
}

fn index_of<T: PartialEq>(haystack: &[T], needle: T) -> usize {
    haystack
        .iter()
        .position(|item| *item == needle)
        .unwrap_or(0)
}

fn wrap(index: usize, len: usize, dir: i32) -> usize {
    if len == 0 {
        return 0;
    }
    (index as isize + dir as isize).rem_euclid(len as isize) as usize
}

fn step(value: u8, dir: i32) -> u8 {
    (value as i32 + dir * RANGE_STEP).clamp(0, 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pane_has_at_least_one_row() {
        for pane in Pane::ALL {
            assert!(!pane.rows().is_empty(), "{:?} has no rows", pane);
        }
    }

    #[test]
    fn row_ids_are_unique_across_every_pane() {
        let mut seen = Vec::new();
        for pane in Pane::ALL {
            for row in pane.rows() {
                assert!(!seen.contains(&row.id), "{:?} listed twice", row.id);
                seen.push(row.id);
            }
        }
    }

    #[test]
    fn pane_stepping_wraps_both_ways() {
        assert_eq!(Pane::Video.stepped(-1), Pane::System);
        assert_eq!(Pane::System.stepped(1), Pane::Video);
        assert_eq!(Pane::Video.stepped(2), Pane::Controls);
    }

    #[test]
    fn choice_rows_offer_one_option_per_value() {
        // A choice row whose option list is shorter than the value set would
        // make some values unreachable from the UI.
        assert_eq!(SCALING_OPTIONS.len(), 3);
        assert_eq!(ASPECT_OPTIONS.len(), 2);
    }

    #[test]
    fn choices_wrap_and_round_trip_to_their_option_index() {
        let mut settings = Settings::default();
        assert_eq!(settings.choice_index(SettingId::Scaling), Some(0));
        assert!(settings.adjust(SettingId::Scaling, -1));
        assert_eq!(settings.scaling, ScaleMode::Integer3x);
        assert_eq!(settings.choice_index(SettingId::Scaling), Some(2));
        assert!(settings.adjust(SettingId::Scaling, 1));
        assert_eq!(settings.scaling, ScaleMode::Fit);
    }

    #[test]
    fn ranges_clamp_at_both_ends_and_report_no_change_there() {
        let mut settings = Settings {
            master_volume: 95,
            ..Settings::default()
        };
        assert!(settings.adjust(SettingId::MasterVolume, 1));
        assert_eq!(settings.master_volume, 100);
        assert!(!settings.adjust(SettingId::MasterVolume, 1));

        settings.master_volume = 5;
        assert!(settings.adjust(SettingId::MasterVolume, -1));
        assert_eq!(settings.master_volume, 0);
        assert!(!settings.adjust(SettingId::MasterVolume, -1));
    }

    #[test]
    fn toggles_flip_and_actions_do_nothing() {
        let mut settings = Settings::default();
        assert!(settings.adjust(SettingId::ShowFps, 1));
        assert!(settings.show_fps);
        assert!(!settings.adjust(SettingId::Rebind, 1));
        assert!(!settings.adjust(SettingId::Version, -1));
    }

    #[test]
    fn only_choice_rows_report_an_option_index() {
        let settings = Settings::default();
        assert!(settings.choice_index(SettingId::ShowFps).is_none());
        assert!(settings.choice_index(SettingId::MasterVolume).is_none());
    }

    #[test]
    fn adjustability_matches_what_navigation_expects() {
        assert!(RowKind::Range.is_adjustable());
        assert!(RowKind::Toggle.is_adjustable());
        assert!(RowKind::Choice { options: &[] }.is_adjustable());
        assert!(!RowKind::Action.is_adjustable());
        assert!(!RowKind::Readout.is_adjustable());
    }
}
