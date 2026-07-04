use crate::config::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(self) -> u16 {
        self.x.saturating_add(self.width.saturating_sub(1))
    }

    pub fn bottom(self) -> u16 {
        self.y.saturating_add(self.height.saturating_sub(1))
    }

    pub fn contains(self, x: u16, y: u16) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }
}

pub fn box_rect(area: Rect, cfg: &AppConfig) -> Rect {
    let width = ((u32::from(area.width) * u32::from(cfg.width_pct)) / 100) as u16;
    let height = ((u32::from(area.height) * u32::from(cfg.height_pct)) / 100) as u16;
    let width = width.clamp(20.min(area.width), area.width);
    let height = height.clamp(5.min(area.height), area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

pub fn box_inner(area: Rect, cfg: &AppConfig) -> Rect {
    let outer = box_rect(area, cfg);
    Rect::new(
        outer.x.saturating_add(1),
        outer.y.saturating_add(1),
        outer.width.saturating_sub(2).max(1),
        outer.height.saturating_sub(2).max(1),
    )
}
