# PLOX - Plugin Load Order eXpert

> Nexus link: <https://www.nexusmods.com/morrowind/mods/54262>

PLOX is a rust re-write of MLOX, a utility tool to sort a modlist topologically according to ordering rules. PLOX also outputs warnings and notes dependent on the mods in your load order. You can use it as a GUI app or as a commandline tool.

Rules are automatically downloaded from the respective Rules repository (see below for details).

Supported games:

- ✅TES3 - Morrowind
- ✅OpenMW
- 🚧Cyberpunk 2077

The PLOX GUI supports a configuration file called `plox.toml` (place next to `plox_gui`) that allows you to customize its behavior. Here's an example of how to use the `plox.toml` file:

Required fields:

```toml
no_rules_download = true
log_level = "debug"
log_to_file = true
```

Optional fields:

```toml
config = "openmw.cfg"
game = "OpenMW"
```

## Rules

> Rules spec taken from [mlox - the elder scrolls Mod Load Order eXpert](https://github.com/mlox/mlox).

Plugins are sorted according to rules. For the rules spec see: [Rules Spec](./docs/Rules_spec.md)

The rules are hosted in their own repository:

- TES3 - Morrowind and OpenMW: <https://github.com/DanaePlays/mlox-rules>
- 🚧Cyberpunk 2077: <https://github.com/rfuzzo/cmop-rules>

**PRs are welcome!**

## Usage

### GUI

1. Download `plox_gui.exe`from  the latest release from <https://github.com/rfuzzo/plox/releases>
2. Place `plox_gui.exe` next to the game's exe
3. Double click `plox_gui.exe` to run

### Commandline Interface

1. Download `plox.exe`from  the latest release from <https://github.com/rfuzzo/plox/releases>
2. Place `plox.exe` next to the game's exe
3. Open a terminal window and run `plox.exe` with a command

## Screenshots

![Screenshot](/assets/screenshot_gui1.png)
![Screenshot](/assets/screenshot_cli1.png)

## Credits

- [MLOX](https://github.com/mlox/mlox)
- [MLOX Rules](https://github.com/DanaePlays/mlox-rules)
- [OpenMw Cfg Crate](https://gitlab.com/bmwinger/openmw-cfg)

## CLI Commands

```txt
Usage: plox.exe [OPTIONS] <COMMAND>

Commands:
  sort    Sorts the current mod load order according to specified rules
  list    Lists the current mod load order
  verify  Verifies integrity of the specified rules
  help    Print this message or the help of the given subcommand(s)

Options:
  -l, --log-level <LOG_LEVEL>  Set the log level, default is "info" [possible values: trace, debug, info, warn, error]
  -g, --game <GAME>            Set the game to evaluate, if no game is specified it will attempt to deduce the game from the current working directory [possible values: morrowind, open-mw, cyberpunk]  
  -n, --non-interactive        Disable user input
  -h, --help                   Print help
  -V, --version                Print version
```

### list

Lists the current mod load order.

```txt
Usage: plox.exe list [OPTIONS]

Options:
  -r, --root <ROOT>      Root game folder (e.g. "Cyberpunk 2077" or "Morrowind"). Default is current working directory
  -c, --config <CONFIG>  (OpenMW only) Path to the openmw.cfg file
  -h, --help             Print help
```

### sort

Sorts the current mod load order according to specified rules

```txt
Usage: plox.exe sort [OPTIONS]

Options:
  -g, --game-folder <GAME_FOLDER>  Root game folder (e.g. "Cyberpunk 2077" or "Morrowind"). Default is current working directory
  -r, --rules-dir <RULES_DIR>      Folder to read sorting rules from. Default is ./mlox for TES3
  -d, --dry-run                    Just print the suggested load order without sorting
  -u, --unstable                   Use the potentially faster unstable sorter
  -n, --no-download                Disable automatic downloading of latest ruleset
  -m, --mod-list <MOD_LIST>        Read the input mods from a file instead of checking the root folder
  -c, --config <CONFIG>            (OpenMW only) Path to the openmw.cfg file
  -h, --help                       Print help
```
