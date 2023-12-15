# Carpet Rules Database

![Logo](assets/logo.png)

A database of carpet rules provided by
[Carpet Mod](https://github.com/gnembon/fabric-carpet) and many of its
[extensions](https://github.com/gnembon/fabric-carpet/wiki/List-of-Carpet-extensions).

## Usage

The site <https://carpet.rubixdev.de/> provides a friendly user interface.

All data is stored in the [`data`](data) directory.
[`data/combined.json`](data/combined.json) is a combined JSON file with all
rules, which should be the only thing you need. The data can be accessed under
<https://data.carpet.rubixdev.de/data/combined.json>.

## A bit of history

To my knowledge, the idea of gathering all carpet rules in one place started
with
[my first attempt at a Carpet config creator](https://github.com/RubixDev/CarpetConfigCreator)
in June 2021. It relied on the Markdown lists most extensions provide, but
proved to be very fragile to upstream changes and just not very reliable and
accurate. For curious minds I have re-published it under
<https://old.carpet.rubixdev.de/>.

In January/February 2022 Crec0 then began work on his
[carpet-rules-database](https://github.com/Crec0/carpet-rules-database) with a
website at <https://carpet-rules.crec.dev/>. He uses a Python script to parse
the rules from the source files of the mod's GitHub/GitLab sources across
branches. This was already much better and more reliable than the Markdown based
approach, but still doesn't always deliver correct results. His site became
relatively widely known in the Carpet community and is currently the standard
for searching Carpet rules. It is even pinned in the Carpet Discord.

His website doesn't provide an interface for creating a Carpet config file
however, and so in August that same year I created a new
[Carpet config creator](https://github.com/RubixDev/carpet-config-creator) based
on Crec0's database. This can now be accessed under
<https://crec.carpet.rubixdev.de/>.

Around the same time I started creating
[my own database](https://github.com/RubixDev/carpet-database/tree/old), based
on the concept of automating running Minecraft instances to get the rule data
from in-game. This should be the most accurate source of information. My
approach used Python scripts to clone the repo sources and run them in Docker
containers with the appropriate Java versions installed. It worked, but there
were a lot more edge cases and difficulties than I expected. Because of this, I
never ended up adding more than ten repos and the config creator never switched
to using this database.

That brings us to now, August 2023, where Crec0's database parser has been
broken for a couple of months, and I was willing to give my database attempt
another chance. At first,
[I used the same Git-based approach](https://github.com/RubixDev/carpet-database/commit/aa35a5a5e8e6894ab27cfa2a8e67544e89c65204)
just without Docker and in Rust instead of Python (but we will just pretend like
that never happened). This of course just surfaced the same issues again, but
now I had another idea: using the mod's released jar binaries. This way we don't
have to worry too much about each mod's specific development setup. So far, this
proved to be mostly straight-forward and accurate, but we'll see what happens in
the long term. My config creator now also uses this new database and is
accessible under <https://carpet.rubixdev.de/>.

## Contributing

To add or update mods or versions of mods, edit the [`mods.toml`](mods.toml)
file. Below is an explanation of the schema. When adding a mod, you probably
want to copy this example and delete everything that isn't needed.

```toml
[[mods]]
# The name of the mod.
# Use the name from the `fabric.mod.json` file or the mod page.
name = ""
# The mod's slug.
# If any source below uses Modrinth, this must be the Modrinth slug.
# Otherwise, use the CurseForge slug or mod id.
slug = ""
# (Optional) If any source below uses CurseForge, set the project id here.
project_id = 0
# (Optional) If any source below uses GitHub, set the repo here `<user>/<repo>`.
repo = ""
# (Optional) A mod-global default for the entrypoint.
# Should be up-to-date with the latest version.
entrypoint = ""
# (Optional) A mod-global default for the settings manager.
# Should be up-to-date with the latest version.
settings_manager = ""
# (Optional) A mod-global default for the settings manager class.
# Should be up-to-date with the latest version.
settings_manager_class = ""
# (Optional) A mod-global default for the rule annotation class.
# Should be up-to-date with the latest version.
rule_annotation_class = ""
# (Optional) A mod-global default for the settings classes.
# Should be up-to-date with the latest version.
settings_classes = [""]
# (Optional) A mod-global default for the `run_client` setting.
# Should be up-to-date with the latest version.
# Defaults to `false`.
run_client = false
# (Optional) Dependencies that are the same for all versions.
# Usually used for libraries like MixinExtras or conditional-mixin.
# Cannot be overwritten per version.
# See the version-specific dependencies for more information.
common_dependencies = []

# You can define one version for every major Minecraft version.
# See the latest one for more information on the available settings.

[mods.versions."1.14"]
minecraft_version = "1.14.4"
printer_version = "v1"
dependencies = ["maven.modrinth:carpet:1.3.7"]
source = { host = "Modrinth", version = "" }
# source = { host = "CurseForge", file_id = 0 }
# source = { host = "GitHub", tag = "", asset = "" }

[mods.versions."1.15"]
minecraft_version = "1.15.2"
printer_version = "v1"
dependencies = ["maven.modrinth:carpet:1.4.8"]
source = { host = "Modrinth", version = "" }
# source = { host = "CurseForge", file_id = 0 }
# source = { host = "GitHub", tag = "", asset = "" }

[mods.versions."1.16"]
minecraft_version = "1.16.5"
printer_version = "v1"
dependencies = ["maven.modrinth:carpet:1.4.44"]
source = { host = "Modrinth", version = "" }
# source = { host = "CurseForge", file_id = 0 }
# source = { host = "GitHub", tag = "", asset = "" }

[mods.versions."1.17"]
minecraft_version = "1.17.1"
printer_version = "v1"
dependencies = ["maven.modrinth:carpet:1.4.57"]
source = { host = "Modrinth", version = "" }
# source = { host = "CurseForge", file_id = 0 }
# source = { host = "GitHub", tag = "", asset = "" }

[mods.versions."1.18"]
minecraft_version = "1.18.2"
printer_version = "v1"
dependencies = ["maven.modrinth:carpet:1.4.69"]
source = { host = "Modrinth", version = "" }
# source = { host = "CurseForge", file_id = 0 }
# source = { host = "GitHub", tag = "", asset = "" }

[mods.versions."1.19"]
minecraft_version = "1.19.4"
printer_version = "v3"
dependencies = ["maven.modrinth:carpet:1.4.101"]
source = { host = "Modrinth", version = "" }
# source = { host = "CurseForge", file_id = 0 }
# source = { host = "GitHub", tag = "", asset = "" }

[mods.versions."1.20"]
# The exact Minecraft to use. This should be the latest for this major MC
# version that the mod supports.
minecraft_version = "1.20.4"
# The printer version. Available printers are `v1`, `v2`, `v3`,
# `magiclib-v1`, and `magiclib-v2.
# Usually, `v1` is used for pre-1.19 mods and `v3` for >=1.19.
# `v2` is for scenarios where the Carpet version already uses the new
# settings manager API, but the mod still uses the old one.
# `magiclib-v1` is the `v1` equivalent for mods that use the
# `WrappedSettingManager` type from MagicLib.
# `magiclib-v2` is the `v3` equivalent for mods that use the
# `WrappedSettingManager` type from MagicLib.
printer_version = "v3"
# (Optional) A version-specific override for the entrypoint.
# Use `""` to unset the mod default.
# The entrypoint should have the value of the main entrypoint in the mod's
# `fabric.mod.json` file, if there is one.
entrypoint = ""
# (Optional) A version-specific override for the settings manager.
# Use `""` to unset the mod default.
# If the mod has a custom settings manager, set this to the full path of the
# field where it is stored.
settings_manager = ""
# (Optional) A version-specific override for the settings manager class.
# Use `""` to unset the mod default.
# If the mod uses a custom type for the settings manager, set it here.
# This is mainly for users of MagicLib, like TCTC.
settings_manager_class = ""
# (Optional) A version-specific override for the rule annotation class.
# Use `""` to unset the mod default.
# This is the class of the `@Rule` annotations used in the settings classes.
# Do not set this unless the mod define its own annotation.
rule_annotation_class = ""
# (Optional) A version-specific override for the settings classes.
# A value must be set either here or for the entire mod.
# This specifies the list of classes that define carpet rules.
# Usually this is just one.
settings_classes = [""]
# (Optional) A version-specific override for the `run_client` setting.
# Usually the mods are run in a server environment.
# However, some mods require a client environment, which can be specified here.
run_client = false
# Additional dependencies this mod needs.
# This always includes the appropriate Carpet version.
# To add mods from Modrinth use `maven.modrinth:<slug>:<version>`.
# To add mods from CurseForge use `curse.maven:<slug>-<project_id>:<file_id>`.
dependencies = ["maven.modrinth:carpet:1.4.128"]
# Where to download the mod from.
# Either Modrinth, CurseForge, or GitHub.
# Modrinth sources can optionally define `filename` to specify a non-primary
# file from the version.
source = { host = "Modrinth", version = "" }
# source = { host = "CurseForge", file_id = 0 }
# source = { host = "GitHub", tag = "", asset = "" }
```
