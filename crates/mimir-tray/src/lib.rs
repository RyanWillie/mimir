//! Mimir Tray - System tray UI for the memory vault
//! 
//! Licensed under AGPL-3.0 to keep derivative UIs open-source

use mimir_core::Result;

/// System tray application
pub struct TrayApp {
    // TODO: Add Tauri app state
}

impl TrayApp {
    /// Create a new tray application
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
    
    /// Run the tray application
    pub async fn run(self) -> Result<()> {
        // TODO: Implement Tauri tray app
        // TODO: Add memory viewer
        // TODO: Add app permission toggles
        // TODO: Add burn buttons
        Ok(())
    }
} 