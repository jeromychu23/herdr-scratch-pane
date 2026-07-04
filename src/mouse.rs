use crate::config::AppConfig;
use crate::geometry::{box_inner, box_rect, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitZone {
    Backdrop,
    Inner,
    Minimize,
    ResizeLeft,
    ResizeRight,
    ResizeTop,
    ResizeBottom,
    ResizeTopLeft,
    ResizeTopRight,
    ResizeBottomLeft,
    ResizeBottomRight,
}

impl HitZone {
    pub fn is_resize(self) -> bool {
        matches!(
            self,
            Self::ResizeLeft
                | Self::ResizeRight
                | Self::ResizeTop
                | Self::ResizeBottom
                | Self::ResizeTopLeft
                | Self::ResizeTopRight
                | Self::ResizeBottomLeft
                | Self::ResizeBottomRight
        )
    }
}

pub fn hit_test(area: Rect, cfg: &AppConfig, x: u16, y: u16) -> HitZone {
    let outer = box_rect(area, cfg);
    if !outer.contains(x, y) {
        return HitZone::Backdrop;
    }

    let right = outer.right();
    let bottom = outer.bottom();
    let minimize_start = right.saturating_sub(5);
    let minimize_end = right.saturating_sub(3);
    if y == outer.y && x >= minimize_start && x <= minimize_end {
        return HitZone::Minimize;
    }

    let on_left = x == outer.x;
    let on_right = x == right;
    let on_top = y == outer.y;
    let on_bottom = y == bottom;

    match (on_left, on_right, on_top, on_bottom) {
        (true, _, true, _) => HitZone::ResizeTopLeft,
        (_, true, true, _) => HitZone::ResizeTopRight,
        (true, _, _, true) => HitZone::ResizeBottomLeft,
        (_, true, _, true) => HitZone::ResizeBottomRight,
        (true, _, _, _) => HitZone::ResizeLeft,
        (_, true, _, _) => HitZone::ResizeRight,
        (_, _, true, _) => HitZone::ResizeTop,
        (_, _, _, true) => HitZone::ResizeBottom,
        _ if box_inner(area, cfg).contains(x, y) => HitZone::Inner,
        _ => HitZone::Backdrop,
    }
}
