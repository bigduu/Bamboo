//! UI Configuration for Copilot authentication and interaction
//!
//! Controls how the authentication flow and other UI elements are presented to the user.

/// Configuration for UI behavior during authentication and operation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiConfig {
    /// Whether to automatically open the browser for device authorization
    pub open_browser: bool,
    
    /// Whether to copy the device code to clipboard
    pub copy_to_clipboard: bool,
    
    /// Whether to use GUI dialogs (vs console output)
    pub use_gui_dialog: bool,
    
    /// Whether to print messages to console
    pub print_console: bool,
}

impl UiConfig {
    /// Full GUI mode - all UI features enabled
    /// 
    /// Best for: Desktop applications, TUI tools, CLI with GUI support
    pub fn gui() -> Self {
        Self {
            open_browser: true,
            copy_to_clipboard: true,
            use_gui_dialog: true,
            print_console: true,
        }
    }
    
    /// Headless mode - console output only, no GUI
    /// 
    /// Best for: Servers, Docker containers, CI/CD environments
    pub fn headless() -> Self {
        Self {
            open_browser: false,
            copy_to_clipboard: false,
            use_gui_dialog: false,
            print_console: true,
        }
    }
    
    /// Silent mode - no output at all
    /// 
    /// Best for: Background services, automated scripts with custom handling
    pub fn silent() -> Self {
        Self {
            open_browser: false,
            copy_to_clipboard: false,
            use_gui_dialog: false,
            print_console: false,
        }
    }
    
    /// Custom configuration with all features disabled
    pub fn none() -> Self {
        Self {
            open_browser: false,
            copy_to_clipboard: false,
            use_gui_dialog: false,
            print_console: false,
        }
    }
    
    /// Builder pattern: enable browser opening
    pub fn with_browser(mut self) -> Self {
        self.open_browser = true;
        self
    }
    
    /// Builder pattern: enable clipboard copy
    pub fn with_clipboard(mut self) -> Self {
        self.copy_to_clipboard = true;
        self
    }
    
    /// Builder pattern: enable GUI dialogs
    pub fn with_gui(mut self) -> Self {
        self.use_gui_dialog = true;
        self
    }
    
    /// Builder pattern: enable console output
    pub fn with_console(mut self) -> Self {
        self.print_console = true;
        self
    }
    
    /// Check if any UI output is enabled
    pub fn has_output(&self) -> bool {
        self.print_console || self.use_gui_dialog
    }
    
    /// Check if interactive features are enabled
    pub fn is_interactive(&self) -> bool {
        self.open_browser || self.use_gui_dialog
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        // Default to GUI mode if features are available, otherwise headless
        #[cfg(feature = "gui")]
        return Self::gui();
        
        #[cfg(not(feature = "gui"))]
        return Self::headless();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_config() {
        let config = UiConfig::gui();
        assert!(config.open_browser);
        assert!(config.copy_to_clipboard);
        assert!(config.use_gui_dialog);
        assert!(config.print_console);
        assert!(config.has_output());
        assert!(config.is_interactive());
    }

    #[test]
    fn test_headless_config() {
        let config = UiConfig::headless();
        assert!(!config.open_browser);
        assert!(!config.copy_to_clipboard);
        assert!(!config.use_gui_dialog);
        assert!(config.print_console);
        assert!(config.has_output());
        assert!(!config.is_interactive());
    }

    #[test]
    fn test_silent_config() {
        let config = UiConfig::silent();
        assert!(!config.open_browser);
        assert!(!config.copy_to_clipboard);
        assert!(!config.use_gui_dialog);
        assert!(!config.print_console);
        assert!(!config.has_output());
        assert!(!config.is_interactive());
    }

    #[test]
    fn test_builder_pattern() {
        let config = UiConfig::none()
            .with_browser()
            .with_console();
        
        assert!(config.open_browser);
        assert!(!config.copy_to_clipboard);
        assert!(!config.use_gui_dialog);
        assert!(config.print_console);
    }
}
