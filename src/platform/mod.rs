use crate::state::{LidState, PowerSource};

pub trait SystemProvider {
    fn get_power_source(&self) -> PowerSource;
    fn get_lid_state(&self) -> LidState;
    fn has_external_display(&self) -> bool;
    fn get_cpu_temp(&self) -> Option<f32>;
    fn suspend_system(&self) -> Result<(), String>;
}

#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
pub use windows::WindowsProvider as DefaultProvider;

#[cfg(not(windows))]
pub mod mock;

#[cfg(not(windows))]
pub use mock::MockProvider as DefaultProvider;
