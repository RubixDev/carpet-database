use std::collections::{hash_map::Entry, HashMap};

use anyhow::Result;
use chrono::{DateTime, Utc};
use ferinth::{structures::version::Version, Ferinth};
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::{
    schema::{MinecraftMajorVersion, Mod, VersionSource},
    CLIENT,
};

static FERINTH: Lazy<Ferinth> = Lazy::new(Ferinth::default);

pub async fn search_updates(mods: &[Mod]) -> Result<()> {
    let mut modrinth_cache = HashMap::new();
    let mut cf_cache = HashMap::new();
    for mod_ in mods {
        for (mc_major, version) in &mod_.versions {
            match &version.source {
                VersionSource::Modrinth { version, .. } => {
                    search_modrinth(&mut modrinth_cache, *mc_major, &mod_.slug, version).await?
                }
                VersionSource::CurseForge { file_id } => {
                    search_curseforge(
                        &mut cf_cache,
                        *mc_major,
                        &mod_.slug,
                        mod_.project_id,
                        *file_id,
                    )
                    .await?
                }
                VersionSource::GitHub { .. } => {
                    println!("\x1b[1;30mskipping GitHub source for {}\x1b[0m", &mod_.slug)
                }
            }
        }
    }

    Ok(())
}

async fn search_modrinth(
    cache: &mut HashMap<String, Vec<Version>>,
    mc_major: MinecraftMajorVersion,
    slug: &str,
    current_version: &str,
) -> Result<()> {
    let versions = match cache.entry(slug.to_owned()) {
        Entry::Vacant(entry) => {
            let mut versions = FERINTH
                .list_versions_filtered(slug, Some(&["fabric"]), None, None)
                .await?;
            versions.sort_by_key(|v| v.date_published);
            entry.insert(versions)
        }
        Entry::Occupied(entry) => entry.into_mut(),
    };

    for minor in mc_major.minors().iter().map(|v| v.as_ref()).rev() {
        if let Some(latest) = versions
            .iter()
            .rev()
            .find(|v| v.game_versions.iter().any(|s| s == minor))
        {
            if latest.version_number == current_version || latest.id == current_version {
                println!("\x1b[1;30m{slug} on {mc_major} is up to date\x1b[0m");
            } else {
                println!(
                    "\x1b[1;32m{slug} has new version for {mc_major}: name '{}', id '{}'\x1b[0m",
                    latest.version_number, latest.id
                );
            }
            return Ok(());
        }
    }

    eprintln!("\x1b[1;31mERROR: no version found for {slug} on {mc_major}\x1b[0m");
    dbg!(versions);

    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
struct CFProject {
    files: Vec<CFFile>,
}

#[derive(Clone, Debug, Deserialize)]
struct CFFile {
    id: i32,
    versions: Vec<String>,
    uploaded_at: DateTime<Utc>,
}

async fn search_curseforge(
    cache: &mut HashMap<i32, CFProject>,
    mc_major: MinecraftMajorVersion,
    slug: &str,
    project_id: i32,
    file_id: i32,
) -> Result<()> {
    let project = match cache.entry(project_id) {
        Entry::Vacant(entry) => {
            let mut project: CFProject = serde_json::from_str(
                &CLIENT
                    .get(format!("https://api.cfwidget.com/{project_id}"))
                    .send()
                    .await?
                    .text()
                    .await?,
            )?;
            project.files.sort_by_key(|f| f.uploaded_at);
            entry.insert(project)
        }
        Entry::Occupied(entry) => entry.into_mut(),
    };

    for minor in mc_major.minors().iter().map(|v| v.as_ref()).rev() {
        if let Some(latest) = project.files.iter().rev().find(|f| {
            f.versions.iter().any(|s| s == "Fabric") && f.versions.iter().any(|s| s == minor)
        }) {
            if latest.id == file_id {
                println!("\x1b[1;30m{slug} on {mc_major} is up to date on CurseForge\x1b[0m");
            } else {
                println!(
                    "\x1b[1;33m{slug} has new version for {mc_major} on CurseForge: id '{}'\x1b[0m",
                    latest.id
                );
            }
            return Ok(());
        }
    }

    eprintln!("\x1b[1;31mERROR: no version found for {slug} on {mc_major}\x1b[0m");
    dbg!(project);

    Ok(())
}
