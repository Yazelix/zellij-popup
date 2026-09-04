pub mod popup_contract;

use popup_contract::TransientPaneGeometry;
use zellij_tile::prelude::FloatingPaneCoordinates;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopupViewport {
    pub columns: usize,
    pub rows: usize,
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
}
