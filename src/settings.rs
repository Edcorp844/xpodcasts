use gtk::gio::{
    Settings,
    prelude::{SettingsExt, SettingsExtManual},
};

use crate::{config, util::external_controls::ExternalControlsMode};

pub trait AppSetting {
    fn schema_id() -> &'static str;

    /// Helper to get the GSettings object for this specific ID
    fn get_settings() -> Settings {
        Settings::new(Self::schema_id())
    }
}

#[derive(Debug, Clone)]
pub struct GenaralSettings {
    pub settings: Settings,
}

impl AppSetting for GenaralSettings {
    fn schema_id() -> &'static str {
        config::APPLICATION_ID
    }
}

impl GenaralSettings {
    pub fn new() -> Self {
        Self {
            settings: Self::get_settings(),
        }
    }

    //-----SETTERS-----------
    pub fn set_libary_auto_sync(&self, value: bool) {
        let _ = self.settings.set_boolean("library-auto-sync", value);
    }

    pub fn set_search_platforms(&self, value: &[&str]) {
        let _ = self.settings.set_strv("search-platforms", value);
    }

    pub fn set_continuous_playback(&self, value: bool) {
        let _ = self.settings.set_boolean("continuous-playback", value);
    }

    pub fn set_skip_foward_seconds(&self, value: i32) {
        let _ = self.settings.set_int("skip-foward-seconds", value);
    }

    pub fn set_skip_backward_seconds(&self, value: i32) {
        let _ = self.settings.set_int("skip-backward-seconds", value);
    }

    pub fn set_external_controls_mode(&self, mode: ExternalControlsMode) {
        let _ = self
            .settings
            .set_enum("external-controls-mode", mode.index() as i32);
    }

    //----------GETTERS--------
    pub fn get_libary_auto_sync(&self) -> bool {
        self.settings.boolean("library-auto-sync")
    }

    pub fn get_search_platforms(&self) -> Vec<String> {
        self.settings
            .strv("search-platforms")
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn get_continuous_playback(&self) -> bool {
        self.settings.boolean("continuous-playback")
    }

    pub fn get_skip_foward_seconds(&self) -> i32 {
        self.settings.int("skip-foward-seconds")
    }

    pub fn get_skip_backward_seconds(&self) -> i32 {
        self.settings.int("skip-backward-seconds")
    }

    pub fn get_external_controls_mode(&self) -> ExternalControlsMode {
        ExternalControlsMode::from_index(self.settings.enum_("external-controls-mode") as u32)
    }

    // ---------utils-----------
    pub fn toggle_search_platform(&self, id: &str, enabled: bool) {
        let mut platforms = self.get_search_platforms();

        if enabled {
            if !platforms.iter().any(|p| p == id) {
                platforms.push(id.to_string());
            }
        } else {
            platforms.retain(|p| p != id);
        }

        let platforms_ref: Vec<&str> = platforms.iter().map(String::as_str).collect();
        self.set_search_platforms(&platforms_ref);
    }
}
