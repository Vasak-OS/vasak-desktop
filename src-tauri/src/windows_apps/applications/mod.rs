pub mod configuration;
pub mod search;
pub mod file_manager;

pub use configuration::create_app_configuration_window;
pub use search::create_search_window;
pub use file_manager::create_file_manager_window;

