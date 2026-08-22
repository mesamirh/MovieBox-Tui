use crate::tui::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashCommand {
    Browse,
    History,
    Favorites,
    List,
    Config,
    DownloadDir,
    Theme,
    Update,
    ToggleUpdate,
    ClearCache,
    Github,
    EnableBdix,
    DisableBdix,
    EnableStreaming,
    DisableStreaming,
    EnableTv,
    DisableTv,
    EnableAddons,
    DisableAddons,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedCommand<'a> {
    Browse,
    History,
    Favorites,
    List,
    Config,
    DownloadDir(&'a str),
    Theme,
    Update,
    ToggleUpdate,
    ClearCache,
    Github,
    EnableBdix,
    DisableBdix,
    EnableStreaming,
    DisableStreaming,
    EnableTv,
    DisableTv,
    EnableAddons,
    DisableAddons,
}

impl SlashCommand {
    pub const ALL: [Self; 19] = [
        Self::Browse,
        Self::History,
        Self::Favorites,
        Self::List,
        Self::Config,
        Self::DownloadDir,
        Self::Theme,
        Self::Update,
        Self::ToggleUpdate,
        Self::ClearCache,
        Self::Github,
        Self::EnableBdix,
        Self::DisableBdix,
        Self::EnableStreaming,
        Self::DisableStreaming,
        Self::EnableTv,
        Self::DisableTv,
        Self::EnableAddons,
        Self::DisableAddons,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Browse => "/browse",
            Self::History => "/history",
            Self::Favorites => "/favorites",
            Self::List => "/list",
            Self::Config => "/config",
            Self::DownloadDir => "/download-dir",
            Self::Theme => "/theme",
            Self::Update => "/update",
            Self::ToggleUpdate => "/toggle-update",
            Self::ClearCache => "/clear-cache",
            Self::Github => "/github",
            Self::EnableBdix => "/enable-bdix",
            Self::DisableBdix => "/disable-bdix",
            Self::EnableStreaming => "/enable-streaming",
            Self::DisableStreaming => "/disable-streaming",
            Self::EnableTv => "/enable-tv",
            Self::DisableTv => "/disable-tv",
            Self::EnableAddons => "/enable-addons",
            Self::DisableAddons => "/disable-addons",
        }
    }

    pub fn description(self, state: &AppState) -> &'static str {
        match self {
            Self::Browse => "Curated, rated & most-watched views",
            Self::History => "Watch history",
            Self::Favorites => "Starred titles",
            Self::List => "Show all TV channels",
            Self::Config => {
                if state.is_addon_mode {
                    "Configure HTTP addons"
                } else {
                    "Configure IPTV playlists"
                }
            }
            Self::DownloadDir => "View or change download folder",
            Self::Theme => "Theme picker",
            Self::Update => "Check for newer release",
            Self::ToggleUpdate => "Toggle automatic update checks",
            Self::ClearCache => "Clear cached data",
            Self::Github => "Open project repository",
            Self::EnableBdix => "Enable BDIX FTP sources",
            Self::DisableBdix => "Disable BDIX FTP sources",
            Self::EnableStreaming => "Enable Streaming mode navigation",
            Self::DisableStreaming => "Disable Streaming mode navigation",
            Self::EnableTv => "Enable TV mode navigation",
            Self::DisableTv => "Disable TV mode navigation",
            Self::EnableAddons => "Enable Addon mode navigation",
            Self::DisableAddons => "Disable Addon mode navigation",
        }
    }

    pub fn is_available(self, state: &AppState) -> bool {
        match self {
            Self::Browse => {
                (state.streaming_enabled && !state.is_tv_mode && !state.is_addon_mode)
                    || (state.addons_enabled && state.is_addon_mode)
            }
            Self::History => {
                (state.streaming_enabled && !state.is_tv_mode && !state.is_addon_mode)
                    || (state.addons_enabled && state.is_addon_mode)
            }
            Self::Favorites => state.favorites_available(),
            Self::List => state.tv_enabled && state.is_tv_mode,
            Self::Config => {
                (state.tv_enabled && state.is_tv_mode)
                    || (state.addons_enabled && state.is_addon_mode)
            }
            Self::EnableBdix => {
                state.streaming_enabled
                    && !state.is_tv_mode
                    && !state.is_addon_mode
                    && !state.bdix_enabled
            }
            Self::DisableBdix => {
                state.streaming_enabled
                    && !state.is_tv_mode
                    && !state.is_addon_mode
                    && state.bdix_enabled
            }
            Self::EnableStreaming => !state.streaming_enabled,
            Self::DisableStreaming => state.streaming_enabled,
            Self::EnableTv => !state.tv_enabled,
            Self::DisableTv => state.tv_enabled,
            Self::EnableAddons => !state.addons_enabled,
            Self::DisableAddons => state.addons_enabled,
            Self::DownloadDir
            | Self::Theme
            | Self::Update
            | Self::ToggleUpdate
            | Self::ClearCache
            | Self::Github => true,
        }
    }

    pub fn suggest(state: &AppState, query: &str) -> Vec<String> {
        let lower = query.to_ascii_lowercase();
        let mut results = Vec::new();

        for cmd in Self::ALL {
            if !cmd.is_available(state) {
                continue;
            }
            let name = cmd.name();
            if name.starts_with(&lower) {
                results.push(name.to_string());
                if cmd == Self::DownloadDir
                    && state.download_dir.is_some()
                    && "/download-dir reset".starts_with(&lower)
                {
                    results.push("/download-dir reset".to_string());
                }
            } else if cmd == Self::DownloadDir
                && state.download_dir.is_some()
                && "/download-dir reset".starts_with(&lower)
            {
                results.push("/download-dir reset".to_string());
            }
        }

        results
    }

    pub fn description_for(suggestion: &str, state: &AppState) -> Option<&'static str> {
        let trimmed = if suggestion.starts_with('/') {
            suggestion.trim()
        } else {
            return None;
        };

        if trimmed == "/download-dir reset" {
            return Some("Reset download folder to default");
        }

        for cmd in Self::ALL {
            if cmd.name() == trimmed {
                return Some(cmd.description(state));
            }
        }
        None
    }

    pub fn parse(input: &str) -> Option<ParsedCommand<'_>> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }

        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let command_name = parts.next()?;
        let arg = parts.next().unwrap_or("").trim();

        match command_name.to_ascii_lowercase().as_str() {
            "/browse" => Some(ParsedCommand::Browse),
            "/history" => Some(ParsedCommand::History),
            "/favorites" => Some(ParsedCommand::Favorites),
            "/list" => Some(ParsedCommand::List),
            "/config" => Some(ParsedCommand::Config),
            "/download-dir" => Some(ParsedCommand::DownloadDir(arg)),
            "/theme" => Some(ParsedCommand::Theme),
            "/update" => Some(ParsedCommand::Update),
            "/toggle-update" => Some(ParsedCommand::ToggleUpdate),
            "/clear-cache" => Some(ParsedCommand::ClearCache),
            "/github" => Some(ParsedCommand::Github),
            "/enable-bdix" => Some(ParsedCommand::EnableBdix),
            "/disable-bdix" => Some(ParsedCommand::DisableBdix),
            "/enable-streaming" => Some(ParsedCommand::EnableStreaming),
            "/disable-streaming" => Some(ParsedCommand::DisableStreaming),
            "/enable-tv" => Some(ParsedCommand::EnableTv),
            "/disable-tv" => Some(ParsedCommand::DisableTv),
            "/enable-addons" => Some(ParsedCommand::EnableAddons),
            "/disable-addons" => Some(ParsedCommand::DisableAddons),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_download_dir_suggest_default_state() {
        let state = AppState::default();
        assert!(state.download_dir.is_none());

        let suggestions = SlashCommand::suggest(&state, "/download-dir");
        assert!(suggestions.contains(&"/download-dir".to_string()));
        assert!(!suggestions.contains(&"/download-dir reset".to_string()));
    }

    #[test]
    fn test_download_dir_suggest_custom_state() {
        let state = AppState {
            download_dir: Some(PathBuf::from("/custom/downloads")),
            ..Default::default()
        };

        let suggestions = SlashCommand::suggest(&state, "/download-dir");
        assert!(suggestions.contains(&"/download-dir".to_string()));
        assert!(suggestions.contains(&"/download-dir reset".to_string()));

        let d_suggestions = SlashCommand::suggest(&state, "/d");
        assert!(d_suggestions.contains(&"/download-dir".to_string()));
        assert!(d_suggestions.contains(&"/download-dir reset".to_string()));
    }

    #[test]
    fn test_download_dir_suggest_subcommand_prefix() {
        let state = AppState {
            download_dir: Some(PathBuf::from("/custom/downloads")),
            ..Default::default()
        };

        let suggestions = SlashCommand::suggest(&state, "/download-dir r");
        assert_eq!(suggestions, vec!["/download-dir reset".to_string()]);

        let suggestions_space = SlashCommand::suggest(&state, "/download-dir ");
        assert_eq!(suggestions_space, vec!["/download-dir reset".to_string()]);
    }

    #[test]
    fn test_download_dir_suggest_mode_parity() {
        let mut state = AppState {
            download_dir: Some(PathBuf::from("/custom/downloads")),
            ..Default::default()
        };

        state.is_addon_mode = false;
        state.is_tv_mode = false;
        let stream_sug = SlashCommand::suggest(&state, "/download-dir");
        assert!(stream_sug.contains(&"/download-dir reset".to_string()));

        state.is_addon_mode = true;
        state.is_tv_mode = false;
        let addon_sug = SlashCommand::suggest(&state, "/download-dir");
        assert!(addon_sug.contains(&"/download-dir reset".to_string()));

        state.is_addon_mode = false;
        state.is_tv_mode = true;
        let tv_sug = SlashCommand::suggest(&state, "/download-dir");
        assert!(tv_sug.contains(&"/download-dir reset".to_string()));
    }

    #[test]
    fn test_favorites_command_parses_and_is_available_by_default() {
        let state = AppState::default();
        assert_eq!(
            SlashCommand::parse("/favorites"),
            Some(ParsedCommand::Favorites)
        );
        assert!(SlashCommand::Favorites.is_available(&state));
        assert_eq!(SlashCommand::Favorites.name(), "/favorites");
    }

    #[test]
    fn test_favorites_command_unavailable_in_tv_mode() {
        let state = AppState {
            is_tv_mode: true,
            ..Default::default()
        };
        assert!(!SlashCommand::Favorites.is_available(&state));
    }

    #[test]
    fn test_download_dir_reset_description() {
        let state = AppState::default();
        assert_eq!(
            SlashCommand::description_for("/download-dir reset", &state),
            Some("Reset download folder to default")
        );
        assert_eq!(
            SlashCommand::description_for("/download-dir", &state),
            Some("View or change download folder")
        );
    }
}
