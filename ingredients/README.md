# Ingredient Library

Predefined application backup rules for Cockup. Include an ingredient by name;
Cockup downloads it automatically. Each YAML in [library/](library/) lists the
paths it backs up.

## Usage

```sh
cockup ingredient search ghostty
cockup ingredient show ghostty
cockup cache refresh ghostty
```

```yaml
destination: backup
symlinks: reference
rules: []
include:
  - ingredient: ghostty
    wrap: ghostty
```

```sh
cockup backup config.yaml --dry-run
cockup backup config.yaml
cockup restore config.yaml
```

See [configuration](../docs/configuration.md#include) and
[commands](../docs/commands.md) for details.

## Catalog

| Ingredient | Description |
| --- | --- |
| [alfred](library/alfred.yaml) | Shortcuts, input settings, and automation preferences. |
| [alt-tab](library/alt-tab.yaml) | Utility preferences and support files. |
| [anytype](library/anytype.yaml) | Preferences and local application data where included. |
| [appcleaner](library/appcleaner.yaml) | Utility preferences and support files. |
| [arc](library/arc.yaml) | Browser preferences and profile data. |
| [bartender](library/bartender.yaml) | Utility preferences and support files. |
| [bettertouchtool](library/bettertouchtool.yaml) | Shortcuts, input settings, and automation preferences. |
| [bruno](library/bruno.yaml) | Development tool preferences and configuration. |
| [calibre](library/calibre.yaml) | Preferences and local application data where included. |
| [chromium](library/chromium.yaml) | Browser preferences and profile data. |
| [cleanshot](library/cleanshot.yaml) | Utility preferences and support files. |
| [craft](library/craft.yaml) | Preferences and local application data where included. |
| [cyberduck](library/cyberduck.yaml) | Utility preferences and support files. |
| [daisydisk](library/daisydisk.yaml) | Utility preferences and support files. |
| [docker](library/docker.yaml) | Development tool preferences and configuration. |
| [fantastical](library/fantastical.yaml) | Preferences and local application data where included. |
| [firefox](library/firefox.yaml) | Browser preferences and profile data. |
| [fork](library/fork.yaml) | Development tool preferences and configuration. |
| [ghostty](library/ghostty.yaml) | Terminal settings and profiles. |
| [github](library/github.yaml) | Development tool preferences and configuration. |
| [google-chrome](library/google-chrome.yaml) | Browser preferences and profile data. |
| [hammerspoon](library/hammerspoon.yaml) | Shortcuts, input settings, and automation preferences. |
| [hazel](library/hazel.yaml) | Utility preferences and support files. |
| [hyper](library/hyper.yaml) | Terminal settings and profiles. |
| [insomnia](library/insomnia.yaml) | Development tool preferences and configuration. |
| [istat-menus](library/istat-menus.yaml) | Utility preferences and support files. |
| [iterm2](library/iterm2.yaml) | Terminal settings and profiles. |
| [joplin](library/joplin.yaml) | Preferences and local application data where included. |
| [karabiner-elements](library/karabiner-elements.yaml) | Shortcuts, input settings, and automation preferences. |
| [keyboard-maestro](library/keyboard-maestro.yaml) | Shortcuts, input settings, and automation preferences. |
| [kitty](library/kitty.yaml) | Terminal settings and profiles. |
| [launchbar](library/launchbar.yaml) | Shortcuts, input settings, and automation preferences. |
| [logseq](library/logseq.yaml) | Preferences and local application data where included. |
| [maccy](library/maccy.yaml) | Utility preferences and support files. |
| [mark-text](library/mark-text.yaml) | Preferences and local application data where included. |
| [microsoft-edge](library/microsoft-edge.yaml) | Browser preferences and profile data. |
| [mongodb-compass](library/mongodb-compass.yaml) | Development tool preferences and configuration. |
| [mullvadvpn](library/mullvadvpn.yaml) | Utility preferences and support files. |
| [obsidian](library/obsidian.yaml) | Preferences and local application data where included. |
| [opera](library/opera.yaml) | Browser preferences and profile data. |
| [orbstack](library/orbstack.yaml) | Development tool preferences and configuration. |
| [podman-desktop](library/podman-desktop.yaml) | Development tool preferences and configuration. |
| [postman](library/postman.yaml) | Development tool preferences and configuration. |
| [raycast](library/raycast.yaml) | Shortcuts, input settings, and automation preferences. |
| [rectangle](library/rectangle.yaml) | Shortcuts, input settings, and automation preferences. |
| [sequel-ace](library/sequel-ace.yaml) | Development tool preferences and configuration. |
| [sourcetree](library/sourcetree.yaml) | Development tool preferences and configuration. |
| [stats](library/stats.yaml) | Utility preferences and support files. |
| [sublime-merge](library/sublime-merge.yaml) | Editor settings, keybindings, and related configuration. |
| [sublime-text](library/sublime-text.yaml) | Editor settings, keybindings, and related configuration. |
| [tabby](library/tabby.yaml) | Terminal settings and profiles. |
| [tableplus](library/tableplus.yaml) | Development tool preferences and configuration. |
| [tailscale](library/tailscale.yaml) | Utility preferences and support files. |
| [ticktick](library/ticktick.yaml) | Preferences and local application data where included. |
| [todoist](library/todoist.yaml) | Preferences and local application data where included. |
| [tor-browser](library/tor-browser.yaml) | Browser preferences and profile data. |
| [transmit](library/transmit.yaml) | Utility preferences and support files. |
| [typora](library/typora.yaml) | Preferences and local application data where included. |
| [visual-studio-code](library/visual-studio-code.yaml) | Editor settings, keybindings, and related configuration. |
| [vivaldi](library/vivaldi.yaml) | Browser preferences and profile data. |
| [vscodium](library/vscodium.yaml) | Editor settings, keybindings, and related configuration. |
| [warp](library/warp.yaml) | Terminal settings and profiles. |
| [wezterm](library/wezterm.yaml) | Terminal settings and profiles. |
| [zed](library/zed.yaml) | Editor settings, keybindings, and related configuration. |
| [zotero](library/zotero.yaml) | Preferences and local application data where included. |

Review an ingredient with `cockup ingredient show NAME` before use. Some include
local data as well as settings; custom configuration locations may need local rules.
