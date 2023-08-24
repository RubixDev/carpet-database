use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet, HashSet},
    env,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{IsTerminal, Write},
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{bail, Context, Result};
use fs_extra::dir::CopyOptions;
use itertools::Itertools;
use once_cell::sync::Lazy;
use reqwest::Client;
use schema::{
    CombinedJson, MinecraftMajorVersion, Mod, ModVersion, ModsToml, PrinterVersion, Rule,
    VersionSource,
};
use serde_json::{json, Map, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::Command,
};
use xshell::{cmd, Shell};

use crate::schema::{RawRule, RulesJson};

mod schema;
#[cfg(feature = "update")]
mod update;

const TERMINAL_CHILD_STDOUT_LINE_COUNT: usize = 15;

static WORKSPACE_DIR: Lazy<PathBuf> =
    Lazy::new(|| Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf());
static ACTIVE_DIR: Lazy<PathBuf> = Lazy::new(|| WORKSPACE_DIR.join("tmp/active"));
static TEMPLATES_DIR: Lazy<PathBuf> = Lazy::new(|| WORKSPACE_DIR.join("tmp/templates"));
static DATA_DIR: Lazy<PathBuf> = Lazy::new(|| WORKSPACE_DIR.join("data"));

static CLIENT: Lazy<Client> = Lazy::new(Client::new);

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(err) = try_main().await {
        std::io::stdout().lock().flush()?;
        return Err(err);
    }
    Ok(())
}

struct Output {
    mod_name: String,
    mod_slug: String,
    mod_url: String,
    minecraft_version: MinecraftMajorVersion,
    version_url: String,
    rules: Vec<RawRule>,
}

async fn try_main() -> Result<()> {
    let sh = Shell::new()?;
    sh.create_dir(&*DATA_DIR)?;
    // make sure the shell's pwd is always the same
    sh.change_dir(&*WORKSPACE_DIR);

    let arg = env::args().nth(1);
    let arg = arg.as_ref();

    let ModsToml { mods } = toml::from_str(include_str!("../mods.toml"))?;

    #[cfg(feature = "update")]
    if arg.map_or(false, |s| s == "update") {
        update::search_updates(&mods).await?;
        return Ok(());
    }

    if arg.map_or(false, |s| s == "get-matrix") {
        let entries = mods
            .iter()
            .map(|mod_| json!({ "slug": mod_.slug }))
            .collect_vec();
        let matrix = json!({ "include": entries });
        let mut file = File::options()
            .append(true)
            .open(env::var("GITHUB_OUTPUT")?)?;
        file.write_all(format!("mod-list={}", serde_json::to_string(&matrix)?).as_bytes())?;
        return Ok(());
    }

    if arg.map_or(true, |s| s != "combine") {
        let used_mc_versions = mods
            .iter()
            .flat_map(|mod_| mod_.versions.values())
            .map(|ver| ver.minecraft_version.to_string())
            .collect::<BTreeSet<_>>();
        gen_template_mods(&sh, used_mc_versions)?;
    }

    let mut outputs: Vec<Output> = vec![];
    for mod_ in &mods {
        if arg
            .and_then(|s| s.strip_prefix("mod:"))
            .map_or(true, |slug| slug == mod_.slug)
        {
            run_mod(
                &sh,
                mod_,
                &mut outputs,
                arg.map_or(false, |s| s == "combine"),
            )
            .await
            .with_context(|| {
                format!("\x1b[1;31mfailed to extract data for mod `{mod_:#?}`\x1b[0m")
            })?;
        }
    }

    if arg.map_or(true, |s| s == "combine") {
        combine(&sh, outputs)?;
    }

    Ok(())
}

fn modify_file(path: impl AsRef<Path>, func: impl FnOnce(String) -> Result<String>) -> Result<()> {
    let content = fs::read_to_string(&path).with_context(|| {
        format!(
            "could not modify file {}: failed to read",
            path.as_ref().display()
        )
    })?;
    let new_content = func(content).with_context(|| {
        format!(
            "could not modify file {}: modify function returned error",
            path.as_ref().display()
        )
    })?;
    fs::write(&path, new_content).with_context(|| {
        format!(
            "could not modify file {}: failed to write",
            path.as_ref().display()
        )
    })?;
    Ok(())
}

fn gen_template_mods(sh: &Shell, mc_versions: BTreeSet<String>) -> Result<()> {
    println!("\x1b[1;36m>>> generating template mods for all Minecraft versions\x1b[0m");

    // prepare directories
    println!("\x1b[36m>> preparing the tmp directory\x1b[0m");
    let tmp_dir = WORKSPACE_DIR.join("tmp");
    let clone_dir = tmp_dir.join("fabricmc.net");
    sh.create_dir(&tmp_dir)?;

    // clone the fabricmc.net source
    println!("\x1b[36m>> cloning the fabricmc.net source\x1b[0m");
    if clone_dir.is_dir() {
        println!("\x1b[34m> directory already exists, skipping clone\x1b[0m");
    } else {
        cmd!(
            sh,
            "git clone https://github.com/FabricMC/fabricmc.net {clone_dir}"
        )
        .run()?;
    }

    // edit the vite config (we only need the generator lib, overwriting the config avoids
    // downloading unnecessary dependencies)
    println!("\x1b[36m>> editing the vite config\x1b[0m");
    let vite_root = clone_dir.join("scripts");
    sh.write_file(
        vite_root.join("vite.config.js"),
        r"
export default {
  build: {
    sourcemap: false,
    minify: false,
    outDir: './dist',
    emptyOutDir: true,
    lib: {
      entry: './src/lib.ts',
      fileName: 'fabric-template-generator',
      name: 'fabric-template-generator',
      formats: ['es'],
    },
  },
}",
    )?;

    // build the library
    println!("\x1b[36m>> building the generator lib\x1b[0m");
    let cd = sh.push_dir(&vite_root);
    cmd!(sh, "deno task buildLib").run()?;
    drop(cd);

    // generate the mod templates for all versions
    println!("\x1b[36m>> generating the template mods\x1b[0m");
    cmd!(sh, "deno run -A gen_template_mods.ts {mc_versions...}").run()?;

    Ok(())
}

async fn run_mod(
    sh: &Shell,
    Mod {
        name,
        slug,
        curseforge_slug,
        project_id,
        repo,
        entrypoint: default_entrypoint,
        settings_manager: default_settings_manager,
        settings_manager_class: default_settings_manager_class,
        rule_annotation_class: default_rule_annotation_class,
        settings_classes: default_settings_classes,
        run_client: default_run_client,
        common_dependencies,
        versions,
    }: &Mod,
    outputs: &mut Vec<Output>,
    combine_only: bool,
) -> Result<()> {
    println!(
        "\x1b[1;32m{0}\n>>> getting rules for '{name}' <<<\n{0}\x1b[0m",
        "-".repeat(50)
    );
    let curseforge_slug = curseforge_slug.as_ref().unwrap_or(slug);
    let mod_url = match versions
        .values()
        .next_back()
        .with_context(|| "mod versions must be non-empty")?
        .source
    {
        VersionSource::Modrinth { .. } => format!("https://modrinth.com/mod/{slug}"),
        VersionSource::CurseForge { .. } => {
            format!("https://curseforge.com/minecraft/mc-mods/{curseforge_slug}")
        }
        VersionSource::GitHub { .. } => format!("https://github.com/{repo}"),
    };
    for (
        mc_major,
        ModVersion {
            minecraft_version,
            printer_version,
            entrypoint,
            settings_manager,
            settings_manager_class,
            rule_annotation_class,
            settings_classes,
            run_client,
            dependencies,
            source,
        },
    ) in versions
    {
        println!("\x1b[1;36m>>> getting rules for '{name}' for Minecraft {mc_major} using {minecraft_version} with printer {printer_version}\x1b[0m");
        let entrypoint = entrypoint
            .as_ref()
            .or(default_entrypoint.as_ref())
            .filter(|s| !s.is_empty());
        let settings_manager = settings_manager
            .as_ref()
            .or(default_settings_manager.as_ref())
            .filter(|s| !s.is_empty());
        let settings_manager_class = settings_manager_class
            .as_ref()
            .or(default_settings_manager_class.as_ref())
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .unwrap_or(match printer_version {
                PrinterVersion::V1 | PrinterVersion::V2 => "carpet.settings.SettingsManager",
                PrinterVersion::V3 => "carpet.api.settings.SettingsManager",
                PrinterVersion::MagicLibV1 | PrinterVersion::MagicLibV2 => {
                    "top.hendrixshen.magiclib.carpet.impl.WrappedSettingManager"
                }
            });
        let rule_annotation_class = rule_annotation_class
            .as_ref()
            .or(default_rule_annotation_class.as_ref())
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .unwrap_or(match printer_version {
                PrinterVersion::V1 | PrinterVersion::V2 => "carpet.settings.Rule",
                PrinterVersion::V3 => "carpet.api.settings.Rule",
                PrinterVersion::MagicLibV1 | PrinterVersion::MagicLibV2 => {
                    "top.hendrixshen.magiclib.carpet.api.annotation.Rule"
                }
            });
        let settings_classes = settings_classes
            .as_ref()
            .or(default_settings_classes.as_ref())
            .with_context(|| "no settings classes specified")?;
        let run_client = run_client.unwrap_or(*default_run_client);
        let dependencies = common_dependencies.iter().chain(dependencies).collect_vec();

        let version_url = match source {
            VersionSource::Modrinth { version, .. } => {
                format!("https://modrinth.com/mod/{slug}/version/{version}")
            }
            VersionSource::CurseForge { file_id } => {
                format!(
                    "https://curseforge.com/minecraft/mc-mods/{curseforge_slug}/files/{file_id}"
                )
            }
            VersionSource::GitHub { tag, .. } => {
                format!("https://github.com/{repo}/releases/tag/{tag}")
            }
        };

        // skip, if data for version already exists for this version and is non-empty
        let output_data_file = DATA_DIR.join(format!("{slug}-{mc_major}.json"));
        let hash = hash((
            mc_major,
            minecraft_version,
            printer_version,
            entrypoint,
            settings_manager,
            settings_manager_class,
            rule_annotation_class,
            settings_classes,
            run_client,
            &dependencies,
            source,
        ));
        if let Some(output) = sh
            .read_file(&output_data_file)
            .ok()
            .and_then(|str| serde_json::from_str::<RulesJson>(&str).ok())
        {
            if output.hash == hash && !output.rules.is_empty() {
                println!("\x1b[34m> data already up-to-date, skipping extraction\x1b[0m");
                outputs.push(Output {
                    mod_name: name.clone(),
                    mod_slug: slug.clone(),
                    mod_url: mod_url.clone(),
                    minecraft_version: *mc_major,
                    version_url,
                    rules: output.rules,
                });
                continue;
            }
        }

        if combine_only {
            bail!("cannot run combine task with outdated data");
        }

        // remove any previous active mod
        println!("\x1b[36m>> removing previous active mod dir\x1b[0m");
        sh.remove_path(&*ACTIVE_DIR)?;

        // copy the respective template
        println!("\x1b[36m>> copying template mod for Minecraft {minecraft_version}\x1b[0m");
        let from = TEMPLATES_DIR.join(minecraft_version.to_string());
        let to = &*ACTIVE_DIR;
        fs_extra::dir::copy(&from, to, &CopyOptions::new().copy_inside(true)).with_context(
            || {
                format!(
                    "couldn't copy template mod from '{}' to '{}'",
                    from.display(),
                    to.display()
                )
            },
        )?;

        // set cwd
        let _cd = sh.push_dir(&*ACTIVE_DIR);

        // write printer class
        println!("\x1b[36m>> writing printer {printer_version} class\x1b[0m");
        let mut mixins = vec![];
        let raw_printer = match printer_version {
            PrinterVersion::V1 => {
                // also add accessor mixin
                sh.write_file(
                    "src/main/java/mixin/SettingsManagerAccessor.java",
                    include_str!("../printers/SettingsManagerAccessor.java"),
                )?;
                mixins.push("SettingsManagerAccessor");

                include_str!("../printers/V1Printer.java")
            }
            PrinterVersion::V2 => include_str!("../printers/V2Printer.java"),
            PrinterVersion::V3 => include_str!("../printers/V3Printer.java"),
            PrinterVersion::MagicLibV1 => include_str!("../printers/MagicLibV1Printer.java"),
            PrinterVersion::MagicLibV2 => include_str!("../printers/MagicLibV2Printer.java"),
        };
        if let Some(settings_manager) = settings_manager {
            let (class_path, field_name) = settings_manager
                .rsplit_once('.')
                .with_context(|| format!("invalid settings_manager path '{settings_manager}'"))?;
            sh.write_file(
                "src/main/java/mixin/PrivateSettingsManagerAccessor.java",
                format!(
                    r###"
package mixin;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

@Mixin({class_path}.class)
public interface PrivateSettingsManagerAccessor {{
    @Accessor(value = "{field_name}", remap = false)
    static {settings_manager_class} getSettingsManager() {{
        throw new AssertionError();
    }}
}}
"###
                ),
            )?;
            mixins.push("PrivateSettingsManagerAccessor");
        }
        let printer = raw_printer
            .replace(
                "SETTINGS_MANAGERS",
                &std::iter::once("carpet.CarpetServer.settingsManager")
                    .chain(
                        settings_manager
                            .is_some()
                            .then_some("mixin.PrivateSettingsManagerAccessor.getSettingsManager()"),
                    )
                    .collect_vec()
                    .join(", "),
            )
            .replace("RULE", rule_annotation_class)
            .replace(
                "SETTINGS_CLASSES",
                &settings_classes
                    .iter()
                    .map(|path| format!("{path}.class"))
                    .collect_vec()
                    .join(", "),
            );
        sh.write_file("src/main/java/Printer.java", printer)?;

        modify_file(
            ACTIVE_DIR.join("src/main/resources/data-extractor.mixins.json"),
            |str| {
                let mut mixins_conf = serde_json::from_str::<Map<String, Value>>(&str)?;
                mixins_conf["package"] = json!("mixin");
                mixins_conf["mixins"] = json!(mixins);
                Ok(serde_json::to_string_pretty(&mixins_conf)?)
            },
        )?;

        // set entrypoints
        println!("\x1b[36m>> setting entrypoints\x1b[0m");
        modify_file(
            ACTIVE_DIR.join("src/main/resources/fabric.mod.json"),
            |str| {
                let mut fabric_conf = serde_json::from_str::<Map<String, Value>>(&str)?;
                let entrypoints = entrypoint
                    .iter()
                    .map(|s| s.as_str())
                    .chain(["carpet.CarpetServer::onGameStarted", "Printer::print"])
                    .collect_vec();
                fabric_conf.insert("entrypoints".to_owned(), json!({ "main": entrypoints }));
                // also remove all dependencies, as some templates use the wrong modid for
                // fabric-api
                fabric_conf.insert("depends".to_owned(), json!({}));
                Ok(serde_json::to_string_pretty(&fabric_conf)?)
            },
        )?;

        // accept EULA
        println!("\x1b[36m>> accepting the EULA\x1b[0m");
        sh.write_file("run/eula.txt", "eula=true")?;

        // add dependencies
        println!("\x1b[36m>> adding dependencies\x1b[0m");
        let main_mod_dep = match source {
            VersionSource::Modrinth { version, filename } => {
                get_modrinth_dep(sh, slug, version, filename).await?
            }
            VersionSource::CurseForge { file_id } => {
                format!("'curse.maven:{slug}-{project_id}:{file_id}'")
            }
            VersionSource::GitHub { tag, asset } => get_github_dep(sh, repo, tag, asset).await?,
        };
        modify_file(ACTIVE_DIR.join("build.gradle"), |str| {
            let extra_deps = dependencies
                .iter()
                .map(|dep| format!("\n    modImplementation '{dep}'"))
                .collect::<String>();
            Ok(str
                + &format!(
                    r###"
repositories {{
    // Modrinth maven
    exclusiveContent {{
        forRepository {{
            maven {{ url = "https://api.modrinth.com/maven" }}
        }}
        filter {{
            includeGroup "maven.modrinth"
        }}
    }}
    // jitpack for GitHub
    maven {{ url = "https://jitpack.io" }}
    // CurseForge maven
    exclusiveContent {{
        forRepository {{
            maven {{ url = "https://cursemaven.com" }}
        }}
        filter {{
            includeGroup "curse.maven"
        }}
    }}
}}

dependencies {{
    modImplementation {main_mod_dep}{extra_deps}
}}
"###,
                ))
        })?;

        // run
        println!("\x1b[36m>> running extraction\x1b[0m");
        let is_terminal = std::io::stdout().lock().is_terminal();
        let mut stdout_log = vec![];
        let mut cmd = Command::new(ACTIVE_DIR.join("gradlew"))
            .arg(if run_client { "runClient" } else { "runServer" })
            .current_dir(&*ACTIVE_DIR)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| "failed to run extraction for mod")?;
        let stdout = cmd.stdout.take().unwrap();
        let mut stderr_reader = cmd.stderr.take().unwrap();
        let mut lines = BufReader::new(stdout).lines();
        print!("\x1b[?7l"); // disable line wrapping
        if is_terminal {
            // make space for output
            print!("{}", "\n".repeat(TERMINAL_CHILD_STDOUT_LINE_COUNT));
        }
        while let Some(line) = lines.next_line().await? {
            if is_terminal {
                // print last n lines of output if in terminal
                print!("\x1b[{TERMINAL_CHILD_STDOUT_LINE_COUNT}A\r\x1b[0J");
                for line in &stdout_log[stdout_log
                    .len()
                    .saturating_sub(TERMINAL_CHILD_STDOUT_LINE_COUNT)..]
                {
                    println!("{line}");
                }
                if stdout_log.len() < TERMINAL_CHILD_STDOUT_LINE_COUNT {
                    print!(
                        "{}",
                        "\n".repeat(TERMINAL_CHILD_STDOUT_LINE_COUNT - stdout_log.len())
                    );
                }
            } else {
                println!("{line}");
            }
            stdout_log.push(line);
        }
        print!("\x1b[?7h"); // enable line wrapping again
        let mut stderr = String::new();
        stderr_reader.read_to_string(&mut stderr).await?;
        if is_terminal {
            // move back up
            print!("\x1b[{TERMINAL_CHILD_STDOUT_LINE_COUNT}A\r\x1b[0J");
        } else {
            println!("\x1b[1;33m------ STDERR ------\x1b[0m\n{stderr}");
        }
        let status = cmd.wait().await?;
        let on_err = || {
            if is_terminal {
                // print full log on failure
                println!(
                    "\x1b[1;31m------ STDOUT ------\x1b[0m\n{}",
                    stdout_log.join("\n")
                );
                println!("\x1b[1;31m------ STDERR ------\x1b[0m\n{stderr}");
            }
        };
        if !status.success() {
            on_err();
            bail!("extraction exited with non-0 exit code: {status}");
        }
        if !sh.path_exists("run/rules.json") {
            on_err();
            bail!("no output rules.json found");
        }

        let mut rules = serde_json::from_str::<Vec<RawRule>>(&sh.read_file("run/rules.json")?)?;
        rules.sort_by_key(|rule| rule.name.clone());
        if rules.is_empty() {
            on_err();
            bail!("extracted rules list is empty");
        }

        // save final json to file
        println!("\x1b[36m>> saving output\x1b[0m");
        let output = RulesJson { hash, rules };
        sh.write_file(output_data_file, serde_json::to_string(&output)?)?;
        outputs.push(Output {
            mod_name: name.clone(),
            mod_slug: slug.clone(),
            mod_url: mod_url.clone(),
            minecraft_version: *mc_major,
            version_url,
            rules: output.rules,
        });
    }
    Ok(())
}

fn hash(state: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    state.hash(&mut hasher);
    hasher.finish()
}

async fn get_modrinth_dep(
    sh: &Shell,
    slug: &str,
    version: &str,
    filename: &Option<String>,
) -> Result<String> {
    let Some(filename) = filename else {
        return Ok(format!("'maven.modrinth:{slug}:{version}'"));
    };

    // download jar manually, because I don't think there is a way to get a file named
    // "CarpetTCTCAddition-1.14.4-2.2.201+8009659-stable.jar" from the maven when the primary file
    // is called "CarpetTCTCAddition-all-2.2.201+8009659-stable.jar"
    let url = format!("https://api.modrinth.com/maven/maven/modrinth/{slug}/{version}/{filename}");
    println!("\x1b[34m> downloading jar from '{url}'\x1b[0m");
    let res = CLIENT.get(url).send().await?;
    if !res.status().is_success() {
        bail!(
            "could not download jar: Modrinth responded with status code {}",
            res.status()
        );
    }
    let bytes = res.bytes().await?;
    sh.write_file("libs/mod.jar", bytes)?;

    Ok("files('libs/mod.jar')".into())
}

async fn get_github_dep(sh: &Shell, repo: &str, tag: &str, asset: &str) -> Result<String> {
    // download jar
    let url = format!("https://github.com/{repo}/releases/download/{tag}/{asset}");
    println!("\x1b[34m> downloading jar from '{url}'\x1b[0m");
    let res = CLIENT.get(url).send().await?;
    if !res.status().is_success() {
        bail!(
            "could not download jar: GitHub responded with status code {}",
            res.status()
        );
    }
    let bytes = res.bytes().await?;
    sh.write_file("libs/mod.jar", bytes)?;

    Ok("files('libs/mod.jar')".into())
}

fn combine(sh: &Shell, outputs: Vec<Output>) -> Result<()> {
    let mut combined: CombinedJson = vec![];

    for Output {
        mod_name,
        mod_slug,
        mod_url,
        minecraft_version,
        version_url,
        rules,
    } in outputs
    {
        for rule in rules {
            let new_rule = Rule {
                name: rule.name,
                description: rule.description,
                type_: rule.type_,
                value: rule.value,
                strict: rule.strict,
                categories: rule.categories,
                options: rule.options,
                extras: rule.extras,
                validators: rule.validators,
                config_files: rule.config_files,
                mod_name: mod_name.clone(),
                mod_slug: mod_slug.clone(),
                mod_url: mod_url.clone(),
                minecraft_versions: vec![minecraft_version],
                version_urls: vec![version_url.clone()],
            };

            let mut did_modify = false;
            for rule in &mut combined {
                if rule.name == new_rule.name
                    && rule.type_ == new_rule.type_
                    && rule.value == new_rule.value
                    && rule.strict == new_rule.strict
                    && rule.categories == new_rule.categories
                    && rule.options == new_rule.options
                    && (rule
                        .validators
                        .iter()
                        .all(|v| new_rule.validators.contains(v))
                        || new_rule
                            .validators
                            .iter()
                            .all(|v| rule.validators.contains(v)))
                    && rule.config_files == new_rule.config_files
                    && rule.mod_name == new_rule.mod_name
                    && rule.mod_slug == new_rule.mod_slug
                    && rule.mod_url == new_rule.mod_url
                {
                    rule.description = new_rule.description.clone();
                    rule.validators = new_rule.validators.clone();
                    rule.minecraft_versions.push(minecraft_version);
                    rule.version_urls.push(version_url.clone());
                    did_modify = true;
                }
            }
            if !did_modify {
                combined.push(new_rule);
            }
        }
    }
    combined.sort_by_key(|rule| rule.name.clone());

    // write file
    sh.write_file(
        DATA_DIR.join("combined.json"),
        serde_json::to_string(&combined)?,
    )?;

    // get and print stats
    let mut count_by_mod: BTreeMap<String, HashSet<String>> = BTreeMap::new();
    let mut count_by_version: BTreeMap<MinecraftMajorVersion, HashSet<String>> = BTreeMap::new();
    let mut count_by_category: BTreeMap<String, HashSet<String>> = BTreeMap::new();
    for rule in &combined {
        count_by_mod
            .entry(rule.mod_name.clone())
            .or_default()
            .insert(rule.name.clone());
        for version in &rule.minecraft_versions {
            count_by_version
                .entry(*version)
                .or_default()
                .insert(rule.name.clone());
        }
        for category in &rule.categories {
            count_by_category
                .entry(category.clone())
                .or_default()
                .insert(rule.name.clone());
        }
    }
    let mut count_by_mod = count_by_mod
        .into_iter()
        .map(|(name, set)| (name, set.len()))
        .collect_vec();
    count_by_mod.sort_by(|a, b| b.1.cmp(&a.1));
    let mut count_by_version = count_by_version
        .into_iter()
        .map(|(name, set)| (name, set.len()))
        .collect_vec();
    count_by_version.sort_by(|a, b| b.1.cmp(&a.1));
    let mut count_by_category = count_by_category
        .into_iter()
        .map(|(name, set)| (name, set.len()))
        .collect_vec();
    count_by_category.sort_by(|a, b| b.1.cmp(&a.1));
    let total_count: usize = count_by_mod.iter().map(|(_, count)| count).sum();

    println!("\x1b[1;32m>>> Rules parsed: {total_count}\x1b[0m");
    println!("\x1b[32m>> Count per mod:\x1b[0m");
    let mut stats_md = format!("**Rules parsed**: {total_count}\n\n");
    stats_md += "Count per mod:\n\n";
    for (mod_name, count) in &count_by_mod {
        println!("\x1b[32m> {mod_name}: {count}\x1b[0m");
        stats_md += &format!("- **{mod_name}**: {count}\n");
    }
    println!("\x1b[32m>> Count per version:\x1b[0m");
    stats_md += "\nCount per version:\n\n";
    for (version, count) in &count_by_version {
        println!("\x1b[32m> {version}: {count}\x1b[0m");
        stats_md += &format!("- **{version}**: {count}\n");
    }
    stats_md += "\nCount per category:\n\n";
    for (category, count) in &count_by_category {
        stats_md += &format!("- `{category}`: {count}\n");
    }
    sh.write_file(WORKSPACE_DIR.join("stats.md"), stats_md)?;

    Ok(())
}
