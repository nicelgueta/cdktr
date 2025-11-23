pub mod admin_panel;
pub mod detail_panel;
pub mod layout;
pub mod main_panel;
/// UI module - panels and rendering components
pub mod sidebar;

pub use admin_panel::AdminPanel;
pub use detail_panel::DetailPanel;
pub use layout::render_layout;
pub use main_panel::MainPanel;
pub use sidebar::Sidebar;
