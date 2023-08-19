use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesJson {
    pub hash: u64,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    /// All other keys are not relevant here, just keep them as is
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}

#[derive(Debug, Clone, Hash, Deserialize)]
pub struct ModsToml {
    pub mods: Vec<Mod>,
}

#[derive(Debug, Clone, Hash, Deserialize)]
pub struct Mod {
    pub name: String,
    pub slug: String,
    /// only for CurseForge
    #[serde(default)]
    pub project_id: i32,
    pub entrypoint: Option<String>,
    pub settings_manager: Option<String>,
    pub settings_classes: Option<Vec<String>>,
    pub versions: BTreeMap<MinecraftMajorVersion, ModVersion>,
}

#[derive(Debug, Clone, Hash, Deserialize)]
pub struct ModVersion {
    pub minecraft_version: MinecraftVersion,
    pub printer_version: PrinterVersion,
    pub entrypoint: Option<String>,
    pub settings_manager: Option<String>,
    pub settings_classes: Option<Vec<String>>,
    /// dependencies other than Fabric API
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub source: VersionSource,
}

#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(tag = "host")]
pub enum VersionSource {
    Modrinth { version: String },
    CurseForge { file_id: i32 },
    GitHub { download_url: String },
}

#[derive(Debug, Clone, Hash, Deserialize, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum PrinterVersion {
    V1,
    V2,
    V3,
}

macro_rules! mc_version_enum {
    ($name:ident; $($variant:ident = $str:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize, strum::Display)]
        pub enum $name {$(
            #[serde(rename = $str)]
            #[strum(serialize = $str)]
            $variant,
        )+}
    };
}

mc_version_enum! {
    MinecraftMajorVersion;
    V1_14 = "1.14",
    V1_15 = "1.15",
    V1_16 = "1.16",
    V1_17 = "1.17",
    V1_18 = "1.18",
    V1_19 = "1.19",
    V1_20 = "1.20",
}

mc_version_enum! {
    MinecraftVersion;
    // commented versions currently generate faulty template mods
    V1_14_4 = "1.14.4",
    V1_15 = "1.15",
    V1_15_1 = "1.15.1",
    V1_15_2 = "1.15.2",
    // V1_16 = "1.16",
    // V1_16_1 = "1.16.1",
    V1_16_2 = "1.16.2",
    V1_16_3 = "1.16.3",
    V1_16_4 = "1.16.4",
    V1_16_5 = "1.16.5",
    // V1_17 = "1.17",
    V1_17_1 = "1.17.1",
    // V1_18 = "1.18",
    V1_18_1 = "1.18.1",
    V1_18_2 = "1.18.2",
    V1_19 = "1.19",
    V1_19_1 = "1.19.1",
    V1_19_2 = "1.19.2",
    V1_19_3 = "1.19.3",
    V1_19_4 = "1.19.4",
    V1_20 = "1.20",
    V1_20_1 = "1.20.1",
}

impl From<MinecraftVersion> for MinecraftMajorVersion {
    fn from(value: MinecraftVersion) -> Self {
        match value {
            MinecraftVersion::V1_14_4 => MinecraftMajorVersion::V1_14,
            MinecraftVersion::V1_15 => MinecraftMajorVersion::V1_15,
            MinecraftVersion::V1_15_1 => MinecraftMajorVersion::V1_15,
            MinecraftVersion::V1_15_2 => MinecraftMajorVersion::V1_15,
            // MinecraftVersion::V1_16 => MinecraftMajorVersion::V1_16,
            // MinecraftVersion::V1_16_1 => MinecraftMajorVersion::V1_16,
            MinecraftVersion::V1_16_2 => MinecraftMajorVersion::V1_16,
            MinecraftVersion::V1_16_3 => MinecraftMajorVersion::V1_16,
            MinecraftVersion::V1_16_4 => MinecraftMajorVersion::V1_16,
            MinecraftVersion::V1_16_5 => MinecraftMajorVersion::V1_16,
            // MinecraftVersion::V1_17 => MinecraftMajorVersion::V1_17,
            MinecraftVersion::V1_17_1 => MinecraftMajorVersion::V1_17,
            // MinecraftVersion::V1_18 => MinecraftMajorVersion::V1_18,
            MinecraftVersion::V1_18_1 => MinecraftMajorVersion::V1_18,
            MinecraftVersion::V1_18_2 => MinecraftMajorVersion::V1_18,
            MinecraftVersion::V1_19 => MinecraftMajorVersion::V1_19,
            MinecraftVersion::V1_19_1 => MinecraftMajorVersion::V1_19,
            MinecraftVersion::V1_19_2 => MinecraftMajorVersion::V1_19,
            MinecraftVersion::V1_19_3 => MinecraftMajorVersion::V1_19,
            MinecraftVersion::V1_19_4 => MinecraftMajorVersion::V1_19,
            MinecraftVersion::V1_20 => MinecraftMajorVersion::V1_20,
            MinecraftVersion::V1_20_1 => MinecraftMajorVersion::V1_20,
        }
    }
}
