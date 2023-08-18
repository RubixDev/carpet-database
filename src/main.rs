use std::{
    fs::{self, Permissions},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use itertools::Itertools;
use lazy_regex::{regex_is_match, regex_replace_all};
use once_cell::sync::Lazy;
use schema::{PrinterVersion, Repo, ReposToml};
use serde_json::{json, Map, Value};
use xshell::{cmd, Shell};

mod schema;

const LOOM_VERSION: &str = "1.3-SNAPSHOT";
const FABRIC_LOADER_VERSION: &str = "0.14.22";

static WORKSPACE_DIR: Lazy<PathBuf> =
    Lazy::new(|| Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf());

fn main() -> Result<()> {
    let sh = Shell::new()?;
    let ReposToml { repos } = toml::from_str(include_str!("../repos.toml"))?;
    sh.create_dir("clone")?;
    sh.change_dir("clone");

    for repo in &repos {
        run_repo(&sh, repo).with_context(|| {
            format!("\x1b[1;31mfailed to extract data for repo `{repo:#?}`\x1b[0m")
        })?;
    }

    Ok(())
}

fn modify_file(path: impl AsRef<Path>, func: impl FnOnce(String) -> Result<String>) -> Result<()> {
    let content = fs::read_to_string(&path)
        .with_context(|| format!("could not modify file {}", path.as_ref().display()))?;
    let new_content = func(content)?;
    fs::write(&path, new_content)?;
    Ok(())
}

fn run_repo(
    sh: &Shell,
    Repo {
        host,
        repo,
        printer_version,
        entrypoint,
        settings_manager,
        settings_files,
        config_files: _,
        branches,
        mappings_override,
    }: &Repo,
) -> Result<()> {
    let repo_dir = sh.create_dir(repo)?;
    if !repo_dir.is_dir() || repo_dir.read_dir()?.next().is_none() {
        cmd!(sh, "git clone https://{host}/{repo} {repo}").run()?;
    }
    let _cd = sh.push_dir(repo);

    for branch in branches {
        println!("\x1b[1;36m>>> extracting data for '{repo}' on branch '{branch}'\x1b[0m");

        // reset git environment and checkout branch
        cmd!(sh, "git checkout .").run()?;
        cmd!(sh, "git clean -fd").run()?;
        cmd!(sh, "git checkout {branch}").run()?;
        cmd!(sh, "git pull --no-rebase").run()?;

        // skip, if data already exists for the current commit
        let output_data_file =
            WORKSPACE_DIR.join(format!("data/{}-{branch}.json", repo.replace('/', "-")));
        let commit = cmd!(sh, "git rev-parse HEAD").read()?;
        let commit = commit.trim();
        if sh
            .read_file(&output_data_file)
            .ok()
            .and_then(|str| serde_json::from_str::<Map<String, Value>>(&str).ok())
            .as_ref()
            .and_then(|map| map.get("commit"))
            .and_then(|val| val.as_str())
            .map_or(false, |hash| hash == commit)
        {
            println!("\x1b[36m> data already on latest version, skipping extraction\x1b[0m");
            continue;
        }

        // use latest gradle wrapper
        fs::write(
            repo_dir.join("gradle/wrapper/gradle-wrapper.properties"),
            include_bytes!("../gradle/gradle-wrapper.properties"),
        )?;
        fs::write(
            repo_dir.join("gradle/wrapper/gradle-wrapper.jar"),
            include_bytes!("../gradle/gradle-wrapper.jar"),
        )?;
        fs::write(
            repo_dir.join("gradlew"),
            include_bytes!("../gradle/gradlew"),
        )?;
        fs::set_permissions(repo_dir.join("gradlew"), Permissions::from_mode(0o755))?;

        // clean up build.gradle
        modify_file(repo_dir.join("build.gradle"), |str| {
            // set loom version
            let str = regex_replace_all!(
                r#"(id\s*['"]fabric-loom['"]\s*version\s*['"]).+?(['"])"#,
                &str,
                |_, start, end| format!("{start}{LOOM_VERSION}{end}")
            );
            // replace `minecraft { accessWidener "..." }` with
            // `loom { accessWidenerPath = file("...") }`
            let str = regex_replace_all!(
                r#"^(minecraft\s*\{[\s\S]*?)accessWidener\s*['"](.*)['"]([\s\S]*?^\})"#m,
                &str,
                |_, start, path, end| format!(
                    "loom {{\n    accessWidenerPath = file('{path}')\n}}\n\n{start}{end}"
                )
            );
            // remove `minecraft.refmapName` usages
            let str = regex_replace_all!(r"^\s*minecraft\.refmapName\s*=.*$"m, &str, "");
            // remove possibly breaking and for us unneeded sections
            let str = regex_replace_all!(
                r"(minecraft|publishing|tasks.*|task\s+sourcesJar.*|(quilt|vine)flower)\s*\{[\s\S]*?\n\}",
                &str,
                ""
            );
            // remove quiltflower/vineflower plugin
            let str = regex_replace_all!(
                r#"id\s*['"]io\.github\.juuxel\.loom-(vine|quilt)flower(-mini)?['"].*"#,
                &str,
                ""
            );
            // replace `modCompile` with the newer`modImplementation`
            let str = regex_replace_all!(r"\bmodCompile\b", &str, "modImplementation");
            // set a duplication strategy for processResources
            let str = regex_replace_all!(
                r"^(processResources\s*\{)([\s\S]*?)(^\})"m,
                &str,
                |_, start, content, end| {
                    if regex_is_match!(r"^\s*duplicatesStrategy\s*=.*$"m, content) {
                        // don't set any if there already is one
                        format!("{start}{content}{end}")
                    } else {
                        format!("{start}{content}    duplicatesStrategy = DuplicatesStrategy.INCLUDE\n{end}")
                    }
                }
            );
            // apply mappings override
            let str = if let Some(version) = mappings_override {
                regex_replace_all!(
                    r#"^(\s*)mappings\s*(['"].*['"]|[^'"]*\{[\s\S]*?\}\s*$)"#m,
                    &str,
                    format!("    mappings 'net.fabricmc:yarn:{version}:v2'")
                )
            } else {
                str
            };
            // add carpet maven repo
            let str = format!(
                "{str}
repositories {{
    maven {{ url = 'https://masa.dy.fi/maven' }}
}}
"
            );

            Ok(str)
        })?;

        // set fabric loader version
        modify_file(repo_dir.join("gradle.properties"), |str| {
            let str = regex_replace_all!(
                r"^(\s*loader_version\s*=\s*).*$"m,
                &str,
                |_, start| format!("{start}{FABRIC_LOADER_VERSION}")
            );
            Ok(str.into_owned())
        })?;

        // write printer class
        let raw_printer = match printer_version {
            PrinterVersion::V1 => include_str!("../printers/V1Printer.java"),
            PrinterVersion::V2 => include_str!("../printers/V2Printer.java"),
        };
        let printer = raw_printer
            .replace(
                "SETTINGS_MANAGER",
                settings_manager
                    .as_ref()
                    .map_or("carpet.CarpetServer.settingsManager", |s| s.as_str()),
            )
            .replace(
                "SETTINGS_FILES",
                &settings_files
                    .iter()
                    .map(|path| format!("{path}.class"))
                    .collect_vec()
                    .join(", "),
            );
        sh.write_file("src/main/java/Printer.java", printer)?;

        // set entrypoints
        modify_file(repo_dir.join("src/main/resources/fabric.mod.json"), |str| {
            let mut fabric_conf = serde_json::from_str::<Map<String, Value>>(&str)?;
            let entrypoints = entrypoint
                .iter()
                .map(|s| s.as_str())
                .chain(["carpet.CarpetServer::onGameStarted", "Printer::print"])
                .collect_vec();
            fabric_conf.insert("entrypoints".to_owned(), json!({ "main": entrypoints }));
            Ok(serde_json::to_string_pretty(&fabric_conf)?)
        })?;

        // make custom settings managers public
        if let Some(settings_manager) = settings_manager {
            let path = repo_dir.join("src/main/java").join(format!(
                "{}.java",
                settings_manager
                    .replace('.', "/")
                    .rsplit_once('/')
                    .unwrap()
                    .0
            ));
            modify_file(path, |str| {
                let str = regex_replace_all!(
                    r"private\b(.*\bstatic\b.*\bSettingsManager\b.+;)",
                    &str,
                    |_, rest| format!("public{rest}")
                );
                Ok(str.into_owned())
            })?;
        }

        // accept EULA
        sh.write_file("run/eula.txt", "eula=true")?;

        // run
        let gradlew = repo_dir.join("gradlew");
        cmd!(sh, "{gradlew} runServer").run()?;
        let rules = serde_json::from_str::<Value>(&sh.read_file("run/rules.json")?)?;

        // save final json to file
        sh.write_file(
            output_data_file,
            serde_json::to_string(&json!({ "commit": commit, "rules": rules }))?,
        )?;
    }

    Ok(())
}
