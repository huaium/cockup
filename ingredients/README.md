# Ingredient Library

The YAML files in `library/` contain Cockup rules only. They have no
`destination` or `symlinks`, so include them from a runnable configuration.
Missing optional paths are skipped. The parent supplies the destination and
symlink policy; an individual include can override the latter.

| Ingredient           | Settings captured                                          | Notes                                                                                            |
| -------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `ghostty`            | XDG and macOS configuration directories.                   | Check custom `XDG_CONFIG_HOME` and external includes.                                            |
| `wezterm`            | `~/.wezterm.lua` and `~/.config/wezterm`.                  | Check `WEZTERM_CONFIG_FILE` overrides.                                                           |
| `karabiner-elements` | Main JSON file and complex modifications.                  | Excludes logs and app state.                                                                     |
| `visual-studio-code` | User settings, keybindings, snippets, and profiles.        | Stable release only; excludes extension binaries and workspace settings.                         |
| `zed`                | Settings, keybindings, themes, and snippets.               | Excludes app data and language-server downloads.                                                 |
| `iterm2`             | Default preferences, application support, and `~/.iterm2`. | Custom settings folders need an edited rule; private preferences and Keychain data are excluded. |
| `rectangle`          | NSUserDefaults preferences plist.                          | Import/export JSON uses a separate, transient path.                                              |
| `bettertouchtool`    | Application support and preferences plist.                 | Includes BetterTouchTool's automatic backup history.                                             |
| `alfred`             | Local preferences package and preference plists.           | A custom sync folder needs an edited rule; caches are excluded.                                  |

The library also includes these 56 cask-checked ingredients (65 total):

| Area                     | Ingredients                                                                                                                                                                                                             |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Terminals                | `hyper`, `kitty`, `tabby`, `warp`                                                                                                                                                                                       |
| Editors                  | `sublime-merge`, `sublime-text`, `vscodium`                                                                                                                                                                             |
| Browsers                 | `arc`, `chromium`, `firefox`, `google-chrome`, `microsoft-edge`, `opera`, `tor-browser`, `vivaldi`                                                                                                                      |
| Writing and productivity | `anytype`, `calibre`, `craft`, `fantastical`, `joplin`, `logseq`, `mark-text`, `obsidian`, `ticktick`, `todoist`, `typora`, `zotero`                                                                                    |
| Development and data     | `bruno`, `docker`, `fork`, `github`, `insomnia`, `mongodb-compass`, `orbstack`, `podman-desktop`, `postman`, `sequel-ace`, `sourcetree`, `tableplus`                                                                    |
| System and utilities     | `alt-tab`, `appcleaner`, `bartender`, `cleanshot`, `cyberduck`, `daisydisk`, `hammerspoon`, `hazel`, `istat-menus`, `keyboard-maestro`, `launchbar`, `maccy`, `mullvadvpn`, `raycast`, `stats`, `tailscale`, `transmit` |

Each YAML lists its exact paths. Some application-support directories contain
profiles or local data as well as settings, particularly browsers and note apps;
check their size and contents before using an ingredient.

Include an ingredient by name to download it from this repository. You can also
save a YAML file alongside your configuration and include it by relative path:

```yaml
symlinks: reference
destination: backup
rules: []
include:
  - ingredient: ghostty
    wrap: ghostty
  - file: ingredients/library/zed.yaml
    wrap: zed
```

Run `cockup backup config.yaml --dry-run` to inspect the selected paths before
writing a backup. Review each YAML file first: settings and workflows can
contain secrets or machine-specific paths, and a copied directory may contain
more than the files named in the table. Quit an app before restoring its files;
macOS preference caching may require logging out before a restored plist is
read. Custom settings locations need a local edit. Named ingredients are cached
under `~/Library/Caches/cockup/ingredients/v1/` and checked for updates after
30 days. Backup stores the exact YAML it used in its manifest for offline
restore and verify. Run `cockup ingredient update NAME` to check GitHub immediately;
if the ingredient is not cached, Cockup asks before downloading it. Run
`cockup ingredient status [NAME]` to inspect cached sources and dates offline.
`cockup ingredient delete NAME [NAME...]` removes one or more YAML/metadata pairs;
`cockup ingredient clean` removes all of `~/Library/Caches/cockup/`.

The original nine ingredients were checked against app documentation and
`brew cat --cask NAME`. The additional 56 were cross-checked against each
cask's `zap` section with `brew cat --cask NAME`.
The cask's `zap` section is an uninstall inventory, not a backup manifest.
Cache, log, saved-state, HTTP-storage, and updater paths were not selected as
standalone targets. An included application-support directory may still contain
such data. WezTerm's `~/.local/share/wezterm`, Karabiner's
`~/.local/share/karabiner`, VS Code's `~/.vscode` extension directory, and Zed's
`~/Library/Application Support/Zed` are outside this configuration-only library.

App references: [Ghostty](https://ghostty.org/docs/config),
[WezTerm](https://wezterm.org/config/files.html),
[Karabiner-Elements](https://karabiner-elements.pqrs.org/docs/json/location/),
[VS Code](https://code.visualstudio.com/docs/configure/settings) and
[profiles](https://code.visualstudio.com/docs/configure/profiles),
[Zed](https://zed.dev/faq), [themes](https://zed.dev/docs/themes), and
[snippets](https://zed.dev/docs/snippets),
[iTerm2](https://iterm2.com/documentation-preferences-general.html),
[Rectangle](https://github.com/rxhanson/Rectangle#preferences-storage),
[BetterTouchTool](https://docs.folivora.ai/docs/getting-started/installation/), and
[Alfred](https://www.alfredapp.com/help/advanced/sync/).
