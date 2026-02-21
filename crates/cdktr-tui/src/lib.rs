use std::io;

// Flux architecture modules
mod actions;
mod app;
mod dispatcher;
mod effects;
mod keyboard;
mod logger;
mod stores;
mod ui;

// Keep existing utilities
mod common;
mod tui;
mod utils;

// Re-export the main entry point
pub use app::App;

/// Main entry point for the TUI application
pub async fn tui_main() -> io::Result<()> {
    // Install color-eyre for better error messages BEFORE terminal init
    if let Err(e) = color_eyre::install() {
        eprintln!("Warning: Failed to install color-eyre: {}", e);
    }

    // Initialize terminal
    let mut terminal = tui::init()?;

    // Create the application and action receiver (this initializes the logger)
    let app_result = App::new();

    let (mut app, action_receiver) = match app_result {
        Ok(app) => app,
        Err(e) => {
            // Make sure to restore terminal before showing error
            let _ = tui::restore();
            eprintln!("Failed to initialize application: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, format!("{}", e)));
        }
    };

    // Run the application
    let result = app.run(&mut terminal, action_receiver).await;

    // Always restore terminal
    let _ = tui::restore();

    if let Err(e) = result {
        eprintln!("Application error: {:?}", e);
        return Err(e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Tests will be added as we develop features
}
