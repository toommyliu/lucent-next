#[derive(
    Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS,
)]
#[serde(rename_all = "kebab-case")]
pub enum RenderingMode {
    #[default]
    Full,
    InterfaceOnly,
    Minimal,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct GameSettings {
    pub animations_enabled: bool,
    pub anti_counter_enabled: bool,
    pub collisions_enabled: bool,
    pub custom_guild: String,
    pub custom_guild_configured: bool,
    pub custom_name: String,
    pub custom_name_configured: bool,
    pub death_ads_visible: bool,
    pub enemy_magnet_enabled: bool,
    pub frame_rate: f64,
    pub infinite_range_enabled: bool,
    pub other_players_visible: bool,
    pub provoke_cell_enabled: bool,
    pub rendering_mode: RenderingMode,
    pub skip_cutscenes_enabled: bool,
    pub walk_speed: f64,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            animations_enabled: true,
            anti_counter_enabled: true,
            collisions_enabled: true,
            custom_guild: String::new(),
            custom_guild_configured: false,
            custom_name: String::new(),
            custom_name_configured: false,
            death_ads_visible: true,
            enemy_magnet_enabled: false,
            frame_rate: 24.0,
            infinite_range_enabled: false,
            other_players_visible: true,
            provoke_cell_enabled: false,
            rendering_mode: RenderingMode::Full,
            skip_cutscenes_enabled: false,
            walk_speed: 8.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct SettingsPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animations_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anti_counter_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collisions_enabled: Option<bool>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<Option<String>>")]
    pub custom_guild: Option<Option<String>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<Option<String>>")]
    pub custom_name: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death_ads_visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enemy_magnet_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub infinite_range_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_players_visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provoke_cell_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendering_mode: Option<RenderingMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_cutscenes_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk_speed: Option<f64>,
}

mod present {
    use serde::{Deserialize, Serialize};

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Option<String>>, D::Error> {
        Option::deserialize(deserializer).map(Some)
    }

    pub fn serialize<S: serde::Serializer>(
        value: &Option<Option<String>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }
}
