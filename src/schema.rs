use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ReposToml {
    pub repos: Vec<Repo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Repo {
    pub host: String,
    pub repo: String,
    pub printer_version: PrinterVersion,
    pub entrypoint: Option<String>,
    pub settings_manager: Option<String>,
    pub settings_files: Vec<String>,
    pub config_files: Vec<String>,
    pub branches: Vec<String>,
    pub mappings_override: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrinterVersion {
    V1,
    V2,
}
