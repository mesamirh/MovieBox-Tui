use crate::tui::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashCommand {
    Settings,
    Browse,
    History,
    Favorites,
    Clear,
    Help,
    List,
    Exit,
}

impl SlashCommand {
    pub const ALL: [Self; 8] = [
        Self::Settings,
        Self::Browse,
        Self::History,
        Self::Favorites,
        Self::Clear,
        Self::Help,
        Self::List,
        Self::Exit,
    ];

    pub const PRIMARY: [Self; 6] = [
        Self::Settings,
        Self::Browse,
        Self::History,
        Self::Favorites,
        Self::Clear,
        Self::Help,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Settings => "/settings",
            Self::Browse => "/browse",
            Self::History => "/history",
            Self::Favorites => "/favorites",
            Self::Clear => "/clear",
            Self::Help => "/help",
            Self::List => "/list",
            Self::Exit => "/exit",
        }
    }

    pub fn description(self, _state: &AppState) -> &'static str {
        match self {
            Self::Settings => "Interactive preferences, content modes & configuration",
            Self::Browse => "Curated, rated & most-watched views",
            Self::History => "Watch history",
            Self::Favorites => "Starred titles",
            Self::Clear => "Clear search results and return to landing",
            Self::Help => "Open interactive keybinding help menu",
            Self::List => "Show all TV channels",
            Self::Exit => "Exit application and restore terminal",
        }
    }

    pub fn is_available(self, state: &AppState) -> bool {
        match self {
            Self::Settings => true,
            Self::Browse | Self::History => state.streaming_enabled && !state.is_tv_mode,
            Self::Favorites => state.favorites_available(),
            Self::List => state.tv_enabled && state.is_tv_mode,
            Self::Clear | Self::Help | Self::Exit => true,
        }
    }

    pub fn suggest(state: &AppState, query: &str) -> Vec<String> {
        let lower = query.to_ascii_lowercase();
        let mut results = Vec::new();

        let candidates: [(&str, Self); 7] = [
            ("/settings", Self::Settings),
            ("/browse", Self::Browse),
            ("/history", Self::History),
            ("/favorites", Self::Favorites),
            ("/clear", Self::Clear),
            ("/help", Self::Help),
            ("/list", Self::List),
        ];

        for (name, cmd) in candidates {
            if !cmd.is_available(state) {
                continue;
            }
            if name.starts_with(&lower) {
                results.push(name.to_string());
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

        if trimmed == "/pref"
            || trimmed == "/preferences"
            || trimmed == "/options"
            || trimmed == "/config"
        {
            return Some(Self::Settings.description(state));
        }
        if trimmed == "/?" {
            return Some(Self::Help.description(state));
        }

        match trimmed {
            "/settings" => Some(Self::Settings.description(state)),
            "/browse" => Some(Self::Browse.description(state)),
            "/history" => Some(Self::History.description(state)),
            "/favorites" => Some(Self::Favorites.description(state)),
            "/clear" => Some(Self::Clear.description(state)),
            "/help" => Some(Self::Help.description(state)),
            "/list" => Some(Self::List.description(state)),
            "/exit" | "/quit" | "/q" => Some(Self::Exit.description(state)),
            _ => None,
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }

        let mut parts = trimmed.split_whitespace();
        let command_name = parts.next()?;

        match command_name.to_ascii_lowercase().as_str() {
            "/settings" | "/config" | "/pref" | "/preferences" | "/options" => Some(Self::Settings),
            "/browse" => Some(Self::Browse),
            "/history" => Some(Self::History),
            "/favorites" => Some(Self::Favorites),
            "/clear" => Some(Self::Clear),
            "/help" | "/?" => Some(Self::Help),
            "/list" => Some(Self::List),
            "/exit" | "/quit" | "/q" => Some(Self::Exit),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_commands_suggest() {
        let state = AppState::default();
        let suggestions = SlashCommand::suggest(&state, "/");
        assert_eq!(
            suggestions,
            vec![
                "/settings".to_string(),
                "/browse".to_string(),
                "/history".to_string(),
                "/favorites".to_string(),
                "/clear".to_string(),
                "/help".to_string(),
            ]
        );

        let s_sug = SlashCommand::suggest(&state, "/s");
        assert_eq!(s_sug, vec!["/settings".to_string()]);

        let c_sug = SlashCommand::suggest(&state, "/c");
        assert_eq!(c_sug, vec!["/clear".to_string()]);

        let t_sug = SlashCommand::suggest(&state, "/t");
        assert!(t_sug.is_empty());
        let p_sug = SlashCommand::suggest(&state, "/p");
        assert!(p_sug.is_empty());

        let d_sug = SlashCommand::suggest(&state, "/download-dir");
        assert!(d_sug.is_empty());

        let tv_state = AppState {
            is_tv_mode: true,
            tv_enabled: true,
            ..Default::default()
        };
        let list_sug = SlashCommand::suggest(&tv_state, "/l");
        assert_eq!(list_sug, vec!["/list".to_string()]);
    }

    #[test]
    fn test_favorites_command_parses_and_is_available_by_default() {
        let state = AppState::default();
        assert_eq!(
            SlashCommand::parse("/favorites"),
            Some(SlashCommand::Favorites)
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
    fn test_core_commands_and_aliases_parse() {
        let state = AppState::default();
        assert_eq!(SlashCommand::ALL.len(), 8);
        assert_eq!(SlashCommand::PRIMARY.len(), 6);
        assert_eq!(SlashCommand::parse("/exit"), Some(SlashCommand::Exit));
        assert_eq!(SlashCommand::parse("/quit"), Some(SlashCommand::Exit));
        assert_eq!(SlashCommand::parse("/q"), Some(SlashCommand::Exit));
        assert!(SlashCommand::Exit.is_available(&state));
        assert_eq!(SlashCommand::Exit.name(), "/exit");

        assert_eq!(SlashCommand::parse("/clear"), Some(SlashCommand::Clear));
        assert!(SlashCommand::Clear.is_available(&state));
        assert_eq!(SlashCommand::Clear.name(), "/clear");

        assert_eq!(SlashCommand::parse("/help"), Some(SlashCommand::Help));
        assert_eq!(SlashCommand::parse("/?"), Some(SlashCommand::Help));
        assert!(SlashCommand::Help.is_available(&state));
        assert_eq!(SlashCommand::Help.name(), "/help");

        assert_eq!(
            SlashCommand::parse("/settings"),
            Some(SlashCommand::Settings)
        );
        assert_eq!(SlashCommand::parse("/config"), Some(SlashCommand::Settings));
        assert_eq!(SlashCommand::parse("/pref"), Some(SlashCommand::Settings));
        assert_eq!(
            SlashCommand::parse("/preferences"),
            Some(SlashCommand::Settings)
        );
        assert_eq!(
            SlashCommand::parse("/options"),
            Some(SlashCommand::Settings)
        );

        assert_eq!(SlashCommand::parse("/list"), Some(SlashCommand::List));
        assert_eq!(SlashCommand::parse("/browse"), Some(SlashCommand::Browse));
        assert_eq!(SlashCommand::parse("/history"), Some(SlashCommand::History));
        assert_eq!(SlashCommand::parse("/theme"), None);
        assert_eq!(SlashCommand::parse("/themes"), None);
        assert_eq!(SlashCommand::parse("/toggle-tv"), None);
        assert_eq!(SlashCommand::parse("/enable-tv"), None);
        assert_eq!(SlashCommand::parse("/disable-tv"), None);
        assert_eq!(SlashCommand::parse("/toggle-addons"), None);
        assert_eq!(SlashCommand::parse("/download-dir"), None);
        assert_eq!(SlashCommand::parse("/download-dir ~/Movies"), None);
        assert_eq!(SlashCommand::parse("/update"), None);
        assert_eq!(SlashCommand::parse("/clear-cache"), None);
        assert_eq!(SlashCommand::parse("/github"), None);
        assert_eq!(SlashCommand::parse("/probe"), None);

        assert!(SlashCommand::suggest(&state, "/toggle").is_empty());
        assert!(SlashCommand::suggest(&state, "/toggle-").is_empty());
        assert!(SlashCommand::suggest(&state, "/probe").is_empty());
    }

    #[test]
    fn test_descriptions_and_aliases() {
        let state = AppState::default();
        assert_eq!(
            SlashCommand::description_for("/settings", &state),
            Some("Interactive preferences, content modes & configuration")
        );
        assert_eq!(
            SlashCommand::description_for("/config", &state),
            Some("Interactive preferences, content modes & configuration")
        );
        assert_eq!(
            SlashCommand::description_for("/pref", &state),
            Some("Interactive preferences, content modes & configuration")
        );
        assert_eq!(
            SlashCommand::description_for("/?", &state),
            Some("Open interactive keybinding help menu")
        );
        assert_eq!(SlashCommand::description_for("/download-dir", &state), None);
        assert_eq!(SlashCommand::description_for("/toggle-tv", &state), None);
        assert_eq!(SlashCommand::description_for("/theme", &state), None);
    }
}
