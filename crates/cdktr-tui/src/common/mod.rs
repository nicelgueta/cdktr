// Reusable components - allow dead code since they may be used in future features
#[allow(dead_code)]
pub mod calendar;
#[allow(dead_code)]
mod data_pane;
#[allow(dead_code)]
mod input;

pub use calendar::{CalendarWidget, centered_rect, render_calendar_below, render_calendar_popup};
