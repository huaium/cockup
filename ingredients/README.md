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
| [1password](library/1password.yaml) | 1Password preferences only; vaults, account state, and browser integrations excluded. |
| [alacritty](library/alacritty.yaml) | Terminal configuration and macOS preference plist. |
| [alfred](library/alfred.yaml) | Shortcuts, input settings, and automation preferences. |
| [alt-tab](library/alt-tab.yaml) | Utility preferences and support files. |
| [android-studio](library/android-studio.yaml) | IDE and emulator preference plists; SDKs and project files excluded. |
| [anki](library/anki.yaml) | Anki profiles, decks, media, and preferences. |
| [anytype](library/anytype.yaml) | Preferences and local application data where included. |
| [appcleaner](library/appcleaner.yaml) | Utility preferences and support files. |
| [arc](library/arc.yaml) | Browser preferences and profile data. |
| [atuin](library/atuin.yaml) | Shell-history tool configuration and themes; history, sync keys, and sessions excluded. |
| [audacity](library/audacity.yaml) | Audio editor configuration and preference plist. |
| [bartender](library/bartender.yaml) | Utility preferences and support files. |
| [bat](library/bat.yaml) | Viewer configuration, custom themes, and syntax definitions. |
| [bbedit](library/bbedit.yaml) | Editor settings, scripts, and application support. |
| [bettertouchtool](library/bettertouchtool.yaml) | Shortcuts, input settings, and automation preferences. |
| [bitwarden](library/bitwarden.yaml) | Preference plists only; vault and account data excluded. |
| [blender](library/blender.yaml) | Versioned user settings, scripts, presets, and add-ons. |
| [brave-browser](library/brave-browser.yaml) | Browser preferences and profiles, including local browsing data. |
| [bruno](library/bruno.yaml) | Development tool preferences and configuration. |
| [btop](library/btop.yaml) | Resource monitor configuration and themes; logs excluded. |
| [caffeine](library/caffeine.yaml) | Keep-awake preferences and application support. |
| [calibre](library/calibre.yaml) | Preferences and local application data where included. |
| [chromium](library/chromium.yaml) | Browser preferences and profile data. |
| [cleanshot](library/cleanshot.yaml) | Utility preferences and support files. |
| [coteditor](library/coteditor.yaml) | Editor preference plist only; container data excluded. |
| [craft](library/craft.yaml) | Preferences and local application data where included. |
| [curl](library/curl.yaml) | HTTP-client preferences; may contain headers, tokens, or credentials. |
| [cyberduck](library/cyberduck.yaml) | Utility preferences and support files. |
| [daisydisk](library/daisydisk.yaml) | Utility preferences and support files. |
| [direnv](library/direnv.yaml) | Global environment-loader configuration and extensions; project env files excluded. |
| [discord](library/discord.yaml) | Discord preferences only; account sessions and local app data excluded. |
| [docker](library/docker.yaml) | Development tool preferences and configuration. |
| [espanso](library/espanso.yaml) | Text-expander configuration, matches, and preferences. |
| [fantastical](library/fantastical.yaml) | Preferences and local application data where included. |
| [firefox](library/firefox.yaml) | Browser preferences and profile data. |
| [fish](library/fish.yaml) | Shell configuration, functions, completions, and universal variables; history excluded. |
| [fork](library/fork.yaml) | Development tool preferences and configuration. |
| [gh](library/gh.yaml) | GitHub CLI preferences and aliases; hosts.yml credentials excluded. |
| [ghostty](library/ghostty.yaml) | Terminal settings and profiles. |
| [gimp](library/gimp.yaml) | GIMP user configuration, brushes, presets, and application support. |
| [git](library/git.yaml) | Global Git configuration, ignore rules, and attributes; credentials excluded. |
| [github](library/github.yaml) | Development tool preferences and configuration. |
| [glab](library/glab.yaml) | GitLab CLI global configuration; may contain authentication tokens. |
| [google-chrome](library/google-chrome.yaml) | Browser preferences and profile data. |
| [hammerspoon](library/hammerspoon.yaml) | Shortcuts, input settings, and automation preferences. |
| [handbrake](library/handbrake.yaml) | HandBrake presets, queue state, and preferences. |
| [hazel](library/hazel.yaml) | Utility preferences and support files. |
| [helix](library/helix.yaml) | Editor configuration, language settings, and themes. |
| [htop](library/htop.yaml) | Process monitor configuration. |
| [hyper](library/hyper.yaml) | Terminal settings and profiles. |
| [iina](library/iina.yaml) | IINA preferences and application configuration. |
| [inkscape](library/inkscape.yaml) | Inkscape settings, extensions, and application support. |
| [insomnia](library/insomnia.yaml) | Development tool preferences and configuration. |
| [istat-menus](library/istat-menus.yaml) | Utility preferences and support files. |
| [iterm2](library/iterm2.yaml) | Terminal settings and profiles. |
| [joplin](library/joplin.yaml) | Preferences and local application data where included. |
| [karabiner-elements](library/karabiner-elements.yaml) | Shortcuts, input settings, and automation preferences. |
| [keepassxc](library/keepassxc.yaml) | Preference plists only; password databases excluded. |
| [keka](library/keka.yaml) | Archive utility preferences and application support. |
| [keyboard-maestro](library/keyboard-maestro.yaml) | Shortcuts, input settings, and automation preferences. |
| [kitty](library/kitty.yaml) | Terminal settings and profiles. |
| [launchbar](library/launchbar.yaml) | Shortcuts, input settings, and automation preferences. |
| [lazygit](library/lazygit.yaml) | Git UI configuration in macOS and XDG locations. |
| [libreoffice](library/libreoffice.yaml) | LibreOffice user configuration, templates, and preference plist. |
| [localsend](library/localsend.yaml) | File-sharing preference plist only; received files excluded. |
| [logseq](library/logseq.yaml) | Preferences and local application data where included. |
| [maccy](library/maccy.yaml) | Utility preferences and support files. |
| [mark-text](library/mark-text.yaml) | Preferences and local application data where included. |
| [microsoft-edge](library/microsoft-edge.yaml) | Browser preferences and profile data. |
| [microsoft-teams](library/microsoft-teams.yaml) | Preference plist only; account sessions and chat data excluded. |
| [mise](library/mise.yaml) | Global tool versions, configuration, and fragments; installed tools and caches excluded. |
| [mongodb-compass](library/mongodb-compass.yaml) | Development tool preferences and configuration. |
| [monitorcontrol](library/monitorcontrol.yaml) | Display brightness and volume preference plist. |
| [mos](library/mos.yaml) | Mouse scrolling preference plist. |
| [mullvadvpn](library/mullvadvpn.yaml) | Utility preferences and support files. |
| [mycli](library/mycli.yaml) | MySQL CLI preferences; history excluded; config may contain connection credentials. |
| [nano](library/nano.yaml) | Editor configuration, keybindings, and custom syntax definitions. |
| [neovim](library/neovim.yaml) | Editor configuration and local plugins; installed runtime data excluded. |
| [notion](library/notion.yaml) | Notion preferences only; offline workspace data and account sessions excluded. |
| [nova](library/nova.yaml) | Editor preference plist only. |
| [nushell](library/nushell.yaml) | Shell scripts, custom commands, and autoload files; history excluded. |
| [obs](library/obs.yaml) | OBS profiles, scenes, and application support; may contain stream credentials and logs. |
| [obsidian](library/obsidian.yaml) | Preferences and local application data where included. |
| [opera](library/opera.yaml) | Browser preferences and profile data. |
| [orbstack](library/orbstack.yaml) | Development tool preferences and configuration. |
| [pgcli](library/pgcli.yaml) | PostgreSQL CLI settings; history excluded; config may contain connection credentials. |
| [podman-desktop](library/podman-desktop.yaml) | Development tool preferences and configuration. |
| [postman](library/postman.yaml) | Development tool preferences and configuration. |
| [raycast](library/raycast.yaml) | Shortcuts, input settings, and automation preferences. |
| [rectangle](library/rectangle.yaml) | Shortcuts, input settings, and automation preferences. |
| [sequel-ace](library/sequel-ace.yaml) | Development tool preferences and configuration. |
| [signal](library/signal.yaml) | Signal preferences only; messages, encryption keys, and account data excluded. |
| [skim](library/skim.yaml) | PDF viewer preferences and bookmarks; PDF documents excluded. |
| [slack](library/slack.yaml) | Slack preferences only; workspace sessions and local app data excluded. |
| [sourcetree](library/sourcetree.yaml) | Development tool preferences and configuration. |
| [spotify](library/spotify.yaml) | Spotify preference plists only; downloaded music and account data excluded. |
| [starship](library/starship.yaml) | Shell prompt configuration; custom STARSHIP_CONFIG needs a local rule. |
| [stats](library/stats.yaml) | Utility preferences and support files. |
| [sublime-merge](library/sublime-merge.yaml) | Editor settings, keybindings, and related configuration. |
| [sublime-text](library/sublime-text.yaml) | Editor settings, keybindings, and related configuration. |
| [tabby](library/tabby.yaml) | Terminal settings and profiles. |
| [tableplus](library/tableplus.yaml) | Development tool preferences and configuration. |
| [tailscale](library/tailscale.yaml) | Utility preferences and support files. |
| [telegram](library/telegram.yaml) | Telegram preferences only; chat databases and account sessions excluded. |
| [the-unarchiver](library/the-unarchiver.yaml) | Archive extraction preference plist. |
| [thunderbird](library/thunderbird.yaml) | Preference plist only; mail profiles and messages excluded. |
| [ticktick](library/ticktick.yaml) | Preferences and local application data where included. |
| [tmux](library/tmux.yaml) | Terminal multiplexer configuration; external sourced files need local rules. |
| [todoist](library/todoist.yaml) | Preferences and local application data where included. |
| [tor-browser](library/tor-browser.yaml) | Browser preferences and profile data. |
| [transmission](library/transmission.yaml) | Transmission preferences and local torrent/resume state; downloaded files excluded. |
| [transmit](library/transmit.yaml) | Utility preferences and support files. |
| [typora](library/typora.yaml) | Preferences and local application data where included. |
| [utm](library/utm.yaml) | Preference plist only; virtual machines excluded. |
| [vim](library/vim.yaml) | Editor configuration, plugins, colors, and runtime files; .viminfo history excluded. |
| [visual-studio-code](library/visual-studio-code.yaml) | Editor settings, keybindings, and related configuration. |
| [vivaldi](library/vivaldi.yaml) | Browser preferences and profile data. |
| [vlc](library/vlc.yaml) | VLC preferences and application configuration. |
| [vscodium](library/vscodium.yaml) | Editor settings, keybindings, and related configuration. |
| [warp](library/warp.yaml) | Terminal settings and profiles. |
| [wezterm](library/wezterm.yaml) | Terminal settings and profiles. |
| [wget](library/wget.yaml) | Download-client preferences; may contain proxy or server credentials. |
| [wireshark](library/wireshark.yaml) | Capture profiles, display filters, and preference plist. |
| [yazi](library/yazi.yaml) | File manager settings, keybindings, themes, and plugins. |
| [zed](library/zed.yaml) | Editor settings, keybindings, and related configuration. |
| [zoom](library/zoom.yaml) | Zoom preference plists only; recordings and account data excluded. |
| [zotero](library/zotero.yaml) | Preferences and local application data where included. |

Review an ingredient with `cockup ingredient show NAME` before use. Some include
local data as well as settings; custom configuration locations may need local rules.
