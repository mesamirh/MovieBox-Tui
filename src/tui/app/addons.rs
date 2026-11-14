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
                self.load_installed_addons_from_config();
                self.state.addon_manager_selected =
                    usize::from(!self.state.installed_addons.is_empty());
                self.state.addon_input_active = false;
                self.state.addon_input_buffer.clear();
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
                    if crate::providers::addons::nuvio::is_direct_script_url(&url) {
                        match crate::providers::addons::nuvio::fetch_single_scraper(&url).await {
                            Ok(installed) => {
                                let name = installed.name.clone();
                                let _ = tokio::task::spawn_blocking(move || {
                                    let mut addons = crate::config::load_addons();
                                    addons.retain(|existing| {
                                        existing.manifest_url != installed.manifest_url
                                    });
                                    addons.push(installed);
                                    crate::config::save_addons(&addons);
                                })
                                .await;
                                sender
                                    .send(Action::SetStatus(format!(
                                        "Installed and enabled {name} (needs Node.js 18+)."
                                    )))
                                    .ok();
                                sender.send(Action::ShowAddonManager).ok();
                            }
                            Err(err) => {
                                sender
                                    .send(Action::SetStatus(format!(
                                        "Error: Nuvio script install failed: {err}"
                                    )))
                                    .ok();
                            }
                        }
                        return;
                    }
                    let url = crate::providers::addons::nuvio::normalize_nuvio_url(&url);
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
                            let _ = tokio::task::spawn_blocking(move || {
                                let mut addons = crate::config::load_addons();
                                addons.retain(|existing| {
                                    existing.manifest_url != installed.manifest_url
                                });
                                addons.push(installed);
                                crate::config::save_addons(&addons);
                            })
                            .await;
                            sender.send(Action::ShowAddonManager).ok();
                        }
                        Err(err) if err == crate::providers::addons::nuvio::REPO_MANIFEST_ERROR => {
                            match crate::providers::addons::nuvio::fetch_repo(&url).await {
                                Ok((repo_base, scrapers)) => {
                                    let count = scrapers.len();
                                    let _ = tokio::task::spawn_blocking(move || {
                                        let mut addons = crate::config::load_addons();
                                        crate::providers::addons::nuvio::merge_repo(
                                            &mut addons,
                                            &repo_base,
                                            &scrapers,
                                        );
                                        crate::config::save_addons(&addons);
                                    })
                                    .await;
                                    sender
                                        .send(Action::SetStatus(format!(
                                            "Installed {count} Nuvio scrapers (all off). Enable the ones you want; needs Node.js 18+."
                                        )))
                                        .ok();
                                    sender.send(Action::ShowAddonManager).ok();
                                }
                                Err(err) => {
                                    sender
                                        .send(Action::SetStatus(format!(
                                            "Error: Nuvio repository install failed: {err}"
                                        )))
                                        .ok();
                                }
                            }
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
