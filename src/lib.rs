pub mod popup_contract;

use std::collections::VecDeque;

use popup_contract::TransientPaneGeometry;
use zellij_tile::prelude::{FloatingPaneCoordinates, PipeSource, TabInfo};

const MAX_TRACKED_CLI_PIPES: usize = 128;

#[derive(Default)]
pub struct CliPipeTracker {
    payload_pipe_ids: VecDeque<String>,
}

impl CliPipeTracker {
    pub fn should_ignore(&mut self, source: &PipeSource, payload: Option<&str>) -> bool {
        let PipeSource::Cli(pipe_id) = source else {
            return false;
        };
        let previous = self.payload_pipe_ids.iter().position(|id| id == pipe_id);
        if let Some(position) = previous {
            self.payload_pipe_ids.remove(position);
        }
        if payload.is_none() {
            return previous.is_some();
        }

        if self.payload_pipe_ids.len() == MAX_TRACKED_CLI_PIPES {
            self.payload_pipe_ids.pop_front();
        }
        self.payload_pipe_ids.push_back(pipe_id.clone());
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopupViewport {
    pub columns: usize,
    pub rows: usize,
}

impl PopupViewport {
    // Floating x is absolute, and Zellij drops passive full-height edge panes from the
    // viewport, so columns come from the display area: a passive left rail must not narrow
    // popups. A passive right-edge pane would cost the right margin; Nova has none.
    pub fn from_tab(tab: &TabInfo) -> Self {
        Self {
            columns: tab.display_area_columns,
            rows: tab.viewport_rows,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeftMarginPaneSnapshot<'a> {
    pub title: &'a str,
    pub exited: bool,
    pub is_floating: bool,
    pub is_suppressed: bool,
    pub columns: usize,
}

pub fn left_margin_is_active(
    panes: &[LeftMarginPaneSnapshot<'_>],
    pane_title: Option<&str>,
) -> bool {
    let Some(pane_title) = pane_title else {
        return true;
    };
    panes.iter().any(|pane| {
        !pane.exited
            && !pane.is_floating
            && !pane.is_suppressed
            && pane.columns > 2
            && pane.title.trim() == pane_title
    })
}

pub fn effective_geometry(
    mut geometry: TransientPaneGeometry,
    left_margin_active: bool,
) -> TransientPaneGeometry {
    if !left_margin_active {
        geometry.left_margin = None;
    }
    geometry
}

pub fn floating_coordinates(
    geometry: TransientPaneGeometry,
    viewport: PopupViewport,
) -> Option<FloatingPaneCoordinates> {
    let (left_margin, right_margin) = if let Some(left_margin) = geometry.left_margin {
        let right_margin = geometry.side_margin.min(viewport.columns.saturating_sub(1));
        (
            left_margin.min(viewport.columns.saturating_sub(right_margin + 1)),
            right_margin,
        )
    } else {
        let side_margin = bounded_margin(geometry.side_margin, viewport.columns);
        (side_margin, side_margin)
    };
    let vertical_margin = bounded_margin(geometry.vertical_margin, viewport.rows);
    FloatingPaneCoordinates::new(
        Some(left_margin.to_string()),
        Some((vertical_margin + 1).to_string()),
        Some(
            viewport
                .columns
                .saturating_sub(left_margin + right_margin)
                .to_string(),
        ),
        Some(
            viewport
                .rows
                .saturating_sub(vertical_margin * 2)
                .to_string(),
        ),
        None,
        None,
    )
}

fn bounded_margin(margin: usize, size: usize) -> usize {
    margin.min(size.saturating_sub(1) / 2)
}

// Test lane: default
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_pipe_payload_trailer_is_ignored() {
        let mut tracker = CliPipeTracker::default();
        let source = PipeSource::Cli("pipe".to_string());

        assert!(!tracker.should_ignore(&source, Some("gitui")));
        assert!(!tracker.should_ignore(&source, Some("stdin chunk")));
        assert!(tracker.should_ignore(&source, None));
    }

    #[test]
    fn fresh_cli_pipe_without_payload_is_not_ignored() {
        let mut tracker = CliPipeTracker::default();
        let source = PipeSource::Cli("pipe".to_string());

        assert!(!tracker.should_ignore(&source, None));
    }

    #[test]
    fn cli_pipe_payload_does_not_affect_another_id() {
        let mut tracker = CliPipeTracker::default();
        let source = PipeSource::Cli("pipe".to_string());
        let other_source = PipeSource::Cli("other".to_string());

        assert!(!tracker.should_ignore(&source, Some("gitui")));
        assert!(!tracker.should_ignore(&other_source, None));
        assert!(tracker.should_ignore(&source, None));
    }

    #[test]
    fn cli_pipe_tracking_is_bounded() {
        let mut tracker = CliPipeTracker::default();

        for id in 0..=MAX_TRACKED_CLI_PIPES {
            let source = PipeSource::Cli(id.to_string());
            assert!(!tracker.should_ignore(&source, Some("gitui")));
            assert!(tracker.payload_pipe_ids.len() <= MAX_TRACKED_CLI_PIPES);
        }

        assert!(!tracker.should_ignore(&PipeSource::Cli("0".to_string()), None));
        assert!(tracker.should_ignore(&PipeSource::Cli(MAX_TRACKED_CLI_PIPES.to_string()), None));
    }

    #[test]
    fn cli_pipe_id_is_forgotten_after_trailer() {
        let mut tracker = CliPipeTracker::default();
        let source = PipeSource::Cli("pipe".to_string());

        assert!(!tracker.should_ignore(&source, Some("")));
        assert!(tracker.should_ignore(&source, None));
        assert!(!tracker.should_ignore(&source, None));
    }

    #[test]
    fn non_cli_pipe_sources_are_unchanged() {
        let mut tracker = CliPipeTracker::default();
        let cli_source = PipeSource::Cli("pipe".to_string());

        assert!(!tracker.should_ignore(&cli_source, Some("gitui")));
        for source in [PipeSource::Keybind, PipeSource::Plugin(1)] {
            assert!(!tracker.should_ignore(&source, Some("gitui")));
            assert!(!tracker.should_ignore(&source, None));
        }
        assert!(tracker.should_ignore(&cli_source, None));
    }

    #[test]
    fn conditional_left_margin_tracks_visible_tiled_pane_width() {
        let pane = |columns, is_floating, is_suppressed| LeftMarginPaneSnapshot {
            title: "sidebar",
            exited: false,
            is_floating,
            is_suppressed,
            columns,
        };

        assert!(left_margin_is_active(&[], None));
        assert!(left_margin_is_active(
            &[pane(32, false, false)],
            Some("sidebar")
        ));
        assert!(!left_margin_is_active(
            &[pane(1, false, false)],
            Some("sidebar")
        ));
        assert!(!left_margin_is_active(
            &[pane(32, true, false), pane(32, false, true)],
            Some("sidebar")
        ));
    }

    #[test]
    fn inactive_conditional_left_margin_restores_symmetric_geometry() {
        let geometry = TransientPaneGeometry {
            side_margin: 1,
            left_margin: Some(33),
            vertical_margin: 0,
        };

        assert_eq!(effective_geometry(geometry, true), geometry);
        assert_eq!(
            effective_geometry(geometry, false),
            TransientPaneGeometry {
                left_margin: None,
                ..geometry
            }
        );
    }

    #[test]
    fn zero_margins_fill_the_viewport_in_cells() {
        let coordinates = floating_coordinates(
            TransientPaneGeometry {
                side_margin: 0,
                left_margin: None,
                vertical_margin: 0,
            },
            PopupViewport {
                columns: 80,
                rows: 24,
            },
        )
        .unwrap();

        assert_eq!(
            coordinates,
            FloatingPaneCoordinates::new(
                Some("0".to_string()),
                Some("1".to_string()),
                Some("80".to_string()),
                Some("24".to_string()),
                None,
                None
            )
            .unwrap()
        );
    }

    #[test]
    fn margin_popup_geometry_uses_fixed_cell_coordinates() {
        let coordinates = floating_coordinates(
            TransientPaneGeometry {
                side_margin: 2,
                left_margin: None,
                vertical_margin: 1,
            },
            PopupViewport {
                columns: 80,
                rows: 24,
            },
        )
        .unwrap();

        assert_eq!(
            coordinates,
            FloatingPaneCoordinates::new(
                Some("2".to_string()),
                Some("2".to_string()),
                Some("76".to_string()),
                Some("22".to_string()),
                None,
                None
            )
            .unwrap()
        );
    }

    #[test]
    fn oversized_margins_clamp_to_visible_pane() {
        let coordinates = floating_coordinates(
            TransientPaneGeometry {
                side_margin: 99,
                left_margin: None,
                vertical_margin: 99,
            },
            PopupViewport {
                columns: 4,
                rows: 3,
            },
        )
        .unwrap();

        assert_eq!(
            coordinates,
            FloatingPaneCoordinates::new(
                Some("1".to_string()),
                Some("2".to_string()),
                Some("2".to_string()),
                Some("1".to_string()),
                None,
                None
            )
            .unwrap()
        );
    }

    #[test]
    fn independent_horizontal_margins_leave_the_requested_rail() {
        let coordinates = floating_coordinates(
            TransientPaneGeometry {
                side_margin: 1,
                left_margin: Some(33),
                vertical_margin: 0,
            },
            PopupViewport {
                columns: 80,
                rows: 24,
            },
        )
        .unwrap();

        assert_eq!(
            coordinates,
            FloatingPaneCoordinates::new(
                Some("33".to_string()),
                Some("1".to_string()),
                Some("46".to_string()),
                Some("24".to_string()),
                None,
                None
            )
            .unwrap()
        );
    }

    #[test]
    fn passive_rail_outside_the_viewport_does_not_narrow_popups() {
        let tab = TabInfo {
            viewport_columns: 48,
            viewport_rows: 24,
            display_area_columns: 80,
            ..Default::default()
        };
        let coordinates = |left_margin| {
            floating_coordinates(
                TransientPaneGeometry {
                    side_margin: 1,
                    left_margin,
                    vertical_margin: 0,
                },
                PopupViewport::from_tab(&tab),
            )
            .unwrap()
        };
        let expected = |x: &str, width: &str| {
            FloatingPaneCoordinates::new(
                Some(x.to_string()),
                Some("1".to_string()),
                Some(width.to_string()),
                Some("24".to_string()),
                None,
                None,
            )
            .unwrap()
        };

        assert_eq!(coordinates(Some(33)), expected("33", "46"));
        assert_eq!(coordinates(None), expected("1", "78"));
    }
}
