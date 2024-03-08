# PLOX - Plugin Load Order eXpert

> 🚧 Work in progress

A small rust utility to sort a modlist topologically according to ordering rules, as wall as output warnings and notes.

Supported games:

- ✅TES3 - Morrowind
- ✅OpenMorrowind
- 🚧Cyberpunk 2077

## Rules

> Rules spec taken from [mlox - the elder scrolls Mod Load Order eXpert](https://github.com/mlox/mlox).

Plugins are sorted according to rules. For the rules spec see: [Rules Spec](./docs/Rules_spec.md)

The rules are hosted in their own repository:

- TES3 - Morrowind and OpenMorrowind: <https://github.com/DanaePlays/mlox-rules>
- 🚧Cyberpunk 2077: <https://github.com/rfuzzo/cmop-rules>

**PRs are welcome!**

## Usage

### 🚧 GUI

1. Download `plox_gui.exe`from  the latest release from <https://github.com/rfuzzo/plox/releases>
2. Place `plox_gui.exe` next to the game's exe
3. Double click `plox_gui.exe` to run

### Commandline Interface

1. Download `plox.exe`from  the latest release from <https://github.com/rfuzzo/plox/releases>
2. Place `plox.exe` next to the game's exe
3. Open a terminal window and run `plox.exe` with a command

```txt
Usage: plox.exe [OPTIONS] <COMMAND>

Commands:
  sort    Sorts the current mod load order according to specified rules
  list    Lists the current mod load order
  verify  Verifies integrity of the specified rules
  help    Print this message or the help of the given subcommand(s)

Options:
  -l, --log-level <LOG_LEVEL>  Set the log level, default is "info" [possible values: trace, debug, info, warn, error]
  -g, --game <GAME>            Set the game to evaluate, if no game is specified it will attempt to deduce the game from the current working directory [possible values: morrowind, open-morrowind, cyberpunk]
  -n, --non-interactive        Disable user input
  -h, --help                   Print help
  -V, --version                Print version
```

### list

Lists the current mod load order.

```txt
Usage: plox.exe list [OPTIONS]

Options:
  -r, --root <ROOT>  Root game folder (e.g. "Cyberpunk 2077" or "Data Files"). Default is current working directory 
  -h, --help         Print help
```

### sort

Sorts the current mod load order according to specified rules

```txt
Usage: plox.exe sort [OPTIONS]

Options:
  -g, --game-folder <GAME_FOLDER>  Root game folder (e.g. "Cyberpunk 2077" or "Data Files"). Default is current working directory
  -r, --rules-dir <RULES_DIR>      Folder to read sorting rules from. Default is ./plox or ./mlox for TES3
  -d, --dry-run                    Just print the suggested load order without sorting
  -u, --unstable                   Use the potentially faster unstable sorter
  -n, --no-download                Disable automatic downloading of latest ruleset
  -m, --mod-list <MOD_LIST>        Read the input mods from a file instead of checking the root folder
  -h, --help                       Print help
```
