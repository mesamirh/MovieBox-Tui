use super::App;
use crate::providers::addons::models::InstalledAddon;
use crate::tui::action::Action;
use crate::tui::overlay::NotificationKind;

impl App {
    pub(super) async fn handle_addons(&mut self, action: Action) -> Option<()> {
        match action {
            Action::SwitchToStreamingMode => {
                self.reset_mode_state();
                self.state.set_mode(crate::tui::state::AppMode::Streaming);
                if self.state.active_provider == crate::providers::models::ProviderKind::Addons {
                    self.state.active_provider = crate::providers::models::ProviderKind::MovieBox;
                }
                self.announce_mode();
                self.persist_config();
            }

            Action::ShowAddonManager => {
                self.reset_transient_overlays();
                self.state.addon_manager_popup = true;
                self.state.input_mode = crate::tui::state::InputMode::Normal;
                self.state.addon_manager_selected = 1;
                self.state.addon_input_active = false;
                self.state.addon_input_buffer.clear();
                self.load_installed_addons_from_config();
            }

            Action::AddonAddManifest(manifest_url) => {
                let url = manifest_url.trim().to_string();
                if url.is_empty() {
                    return None;
                }
                if !self.state.addons_enabled {
                    self.state.addons_enabled = true;
                    self.persist_config();
                }
                self.state.set_status_long("Verifying addon manifest...");
                let client = self.service.addon_client.clone();
                let sender = self.action_sender.clone();

                tokio::spawn(async move {
                    match client.fetch_manifest(&url).await {
                        Ok(manifest) => {
                            let installed = InstalledAddon::from_manifest(url.clone(), &manifest);
                            sender
                                .send(Action::SetStatus(format!(
                                    "Installed {} v{}",
                                    installed.name,
                                    installed.version.as_deref().unwrap_or("1.0.0")
                                )))
                                .ok();
                            let mut addons = crate::config::load_addons();
                            addons
                                .retain(|existing| existing.manifest_url != installed.manifest_url);
                            addons.push(installed);
                            crate::config::save_addons(&addons);
                            sender.send(Action::ShowAddonManager).ok();
                        }
                        Err(err) => {
                            sender
                                .send(Action::SetStatus(format!(
                                    "Error: Addon install failed: {err}"
                                )))
                                .ok();
                        }
                    }
                });
            }

            Action::AddonToggleEnabled(index) => {
                if index < self.state.installed_addons.len() {
                    if self.state.installed_addons[index].is_core() {
                        self.state.notify(
                            NotificationKind::Info,
                            "Core Provider",
                            "Cinemeta is the primary metadata provider and is locked enabled.",
                        );
                        return None;
                    }
                    self.state.installed_addons[index].enabled =
                        !self.state.installed_addons[index].enabled;
                    self.save_installed_addons();
                }
            }

            Action::AddonRemove(index) => {
                if index < self.state.installed_addons.len() {
                    if self.state.installed_addons[index].is_core() {
                        self.state.notify(
                            NotificationKind::Warning,
                            "Protected Addon",
                            "Cinemeta is the core metadata provider and cannot be uninstalled.",
                        );
                        return None;
                    }
                    let removed = self.state.installed_addons.remove(index);
                    self.save_installed_addons();
                    self.state.notify(
                        NotificationKind::Info,
                        "Addon Removed",
                        format!("Removed {}", removed.name),
                    );
                    if self.state.addon_manager_selected > self.state.installed_addons.len() {
                        self.state.addon_manager_selected = self.state.installed_addons.len();
                    }
                }
            }

            Action::AddonInputToggle(active) => {
                self.state.addon_input_active = active;
                self.state.addon_input_buffer.clear();
            }

            _ => return None,
        }
        None
    }
}
