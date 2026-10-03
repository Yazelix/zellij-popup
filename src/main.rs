use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use yazelix_zellij_popup::{
    effective_geometry, floating_coordinates, left_margin_is_active,
    popup_contract::{
        resolve_transient_toggle_plan_by_identity, select_transient_pane_by_identity,
        should_restart_popup_for_cwd, ConfiguredPopupSpecs, PopupMessageRequestError,
        TransientPaneGeometry, TransientPaneSnapshot, TransientPopupAction,
        TransientPopupCommandHook, TransientPopupPipeRequest, TransientPopupToggleCloseBehavior,
        TransientTogglePlan,
    },
    LeftMarginPaneSnapshot, PopupViewport,
};
use zellij_tile::prelude::*;

const RESULT_CLOSED: &str = "closed";
const RESULT_CLOSED_FLOATING_CLEANUP_FAILED: &str = "closed_floating_cleanup_failed";
const RESULT_DENIED: &str = "permissions_denied";
const RESULT_FOCUSED: &str = "focused";
const RESULT_HIDDEN: &str = "hidden";
const RESULT_HIDDEN_FLOATING_CLEANUP_FAILED: &str = "hidden_floating_cleanup_failed";
const RESULT_INVALID_CONFIG: &str = "invalid_config";
const RESULT_INVALID_PAYLOAD: &str = "invalid_payload";
const RESULT_MISSING: &str = "missing";
const RESULT_MISSING_CONFIG: &str = "missing_config";
const RESULT_NOT_READY: &str = "not_ready";
const RESULT_OPENED: &str = "opened";

#[derive(Default)]
struct State {
    active_tab: Option<ActiveTab>,
    terminal_panes_by_tab: HashMap<usize, Vec<TerminalPane>>,
    popup_launch_cwds: HashMap<PaneId, String>,
    popup_geometries: HashMap<PaneId, TransientPaneGeometry>,
    left_margin_active_by_tab: HashMap<usize, bool>,
    initial_cwd: PathBuf,
    permissions_granted: bool,
    popup_specs: ConfiguredPopupSpecs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveTab {
    position: usize,
    viewport: PopupViewport,
    floating_panes_visible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TerminalPane {
    pane_id: PaneId,
    title: String,
    terminal_command: Option<String>,
    exited: bool,
    is_focused: bool,
    is_floating: bool,
    is_suppressed: bool,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        set_selectable(false);
        self.initial_cwd = get_plugin_ids().initial_cwd;
        self.popup_specs = ConfiguredPopupSpecs::from_configuration(&configuration);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::OpenTerminalsOrPlugins,
            PermissionType::RunCommands,
            PermissionType::ReadCliPipes,
        ]);
        subscribe(&[
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::PaneClosed,
            EventType::PermissionRequestResult,
            EventType::PluginConfigurationChanged,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tabs) => {
                let previous_tab = self.active_tab;
                self.active_tab = tabs.iter().find(|tab| tab.active).map(|tab| ActiveTab {
                    position: tab.position,
                    viewport: PopupViewport {
                        columns: tab.viewport_columns,
                        rows: tab.viewport_rows,
                    },
                    floating_panes_visible: tab.are_floating_panes_visible,
                });
                if self.active_tab != previous_tab {
                    self.reflow_visible_popups();
                }
            }
            Event::PaneUpdate(pane_manifest) => {
                let previous_left_margin_active = self
                    .active_tab
                    .and_then(|tab| self.left_margin_active_by_tab.get(&tab.position).copied());
                self.terminal_panes_by_tab = build_terminal_panes_by_tab(&pane_manifest);
                self.left_margin_active_by_tab = build_left_margin_active_by_tab(
                    &pane_manifest,
                    self.popup_specs.left_margin_pane_title(),
                );
                let left_margin_changed = self.active_tab.is_some_and(|tab| {
                    previous_left_margin_active
                        != self.left_margin_active_by_tab.get(&tab.position).copied()
                });
                if left_margin_changed {
                    self.reflow_visible_popups();
                }
            }
            Event::PaneClosed(pane_id) => {
                self.popup_launch_cwds.remove(&pane_id);
                self.popup_geometries.remove(&pane_id);
            }
            Event::PermissionRequestResult(status) => {
                self.permissions_granted = status == PermissionStatus::Granted;
            }
            Event::PluginConfigurationChanged(configuration) => {
                self.popup_specs = ConfiguredPopupSpecs::from_configuration(&configuration);
            }
            _ => {}
        }

        false
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        let request = match self
            .popup_specs
            .request_from_message(&pipe_message.name, pipe_message.payload.as_deref())
        {
            Ok(request) => request,
            Err(PopupMessageRequestError::UnknownAction) => return false,
            Err(PopupMessageRequestError::InvalidPayload) => {
                self.respond(&pipe_message, RESULT_INVALID_PAYLOAD);
                return false;
            }
            Err(PopupMessageRequestError::InvalidConfiguredSpec(_)) => {
                self.respond(&pipe_message, RESULT_INVALID_CONFIG);
                return false;
            }
            Err(PopupMessageRequestError::MissingConfiguredSpec(_)) => {
                self.respond(&pipe_message, RESULT_MISSING_CONFIG);
                return false;
            }
        };

        self.handle_transient_popup(&pipe_message, request);
        false
    }

    fn render(&mut self, _rows: usize, _cols: usize) {}
}

impl State {
    fn handle_transient_popup(
        &mut self,
        pipe_message: &PipeMessage,
        request: TransientPopupPipeRequest,
    ) {
        let Some(active_tab) = self.ensure_ready(pipe_message) else {
            return;
        };

        let terminal_panes = self
            .terminal_panes_by_tab
            .get(&active_tab.position)
            .cloned()
            .unwrap_or_default();
        let snapshots: Vec<TransientPaneSnapshot<'_, PaneId>> = terminal_panes
            .iter()
            .map(|pane| pane.transient_snapshot())
            .collect();

        let fallback_cwd = self.launch_fallback_cwd(active_tab.position);
        let request_cwd = request
            .launch_plan(&fallback_cwd)
            .map(|plan| plan.cwd)
            .unwrap_or_else(|| fallback_cwd.clone());

        match request.action {
            TransientPopupAction::Toggle => {
                match resolve_transient_toggle_plan_by_identity(
                    &snapshots,
                    request.spec.identity(),
                    active_tab.floating_panes_visible,
                ) {
                    TransientTogglePlan::Open => {
                        self.displace_other_configured_popups(
                            &request,
                            &snapshots,
                            None,
                            &fallback_cwd,
                        );
                        self.open_popup(pipe_message, &request, &fallback_cwd, active_tab.viewport)
                    }
                    TransientTogglePlan::Focus(pane_id) => {
                        self.displace_other_configured_popups(
                            &request,
                            &snapshots,
                            Some(pane_id),
                            &fallback_cwd,
                        );
                        let is_suppressed = snapshots
                            .iter()
                            .find(|pane| pane.pane_id == pane_id)
                            .is_some_and(|pane| pane.is_suppressed);
                        self.show_popup(
                            pipe_message,
                            &request,
                            pane_id,
                            is_suppressed,
                            &fallback_cwd,
                            &request_cwd,
                            active_tab.viewport,
                        );
                    }
                    TransientTogglePlan::ToggleFocused(pane_id) => {
                        self.displace_other_configured_popups(
                            &request,
                            &snapshots,
                            Some(pane_id),
                            &fallback_cwd,
                        );
                        let exited = snapshots
                            .iter()
                            .any(|pane| pane.pane_id == pane_id && pane.exited);
                        match (exited, request.spec.toggle_close_behavior) {
                            (true, _) | (_, TransientPopupToggleCloseBehavior::Close) => {
                                self.close_popup(
                                    pipe_message,
                                    pane_id,
                                    request.spec.on_close.as_ref(),
                                    &request_cwd,
                                );
                            }
                            (false, TransientPopupToggleCloseBehavior::Hide) => {
                                self.hide_popup(
                                    pipe_message,
                                    pane_id,
                                    request.spec.on_hide.as_ref(),
                                    &request_cwd,
                                );
                            }
                        }
                    }
                }
            }
            TransientPopupAction::Open => {
                self.displace_other_configured_popups(&request, &snapshots, None, &fallback_cwd);
                self.open_popup(pipe_message, &request, &fallback_cwd, active_tab.viewport);
            }
            TransientPopupAction::Focus => {
                match select_transient_pane_by_identity(&snapshots, request.spec.identity()) {
                    Some(pane) => {
                        self.displace_other_configured_popups(
                            &request,
                            &snapshots,
                            Some(pane.pane_id),
                            &fallback_cwd,
                        );
                        let is_suppressed = snapshots
                            .iter()
                            .find(|candidate| candidate.pane_id == pane.pane_id)
                            .is_some_and(|candidate| candidate.is_suppressed);
                        self.show_popup(
                            pipe_message,
                            &request,
                            pane.pane_id,
                            is_suppressed,
                            &fallback_cwd,
                            &request_cwd,
                            active_tab.viewport,
                        );
                    }
                    None => self.respond(pipe_message, RESULT_MISSING),
                }
            }
            TransientPopupAction::Replace => {
                let pane = select_transient_pane_by_identity(&snapshots, request.spec.identity());
                self.displace_other_configured_popups(
                    &request,
                    &snapshots,
                    pane.map(|pane| pane.pane_id),
                    &fallback_cwd,
                );
                if let Some(pane) = pane {
                    self.replace_popup(
                        pipe_message,
                        &request,
                        pane.pane_id,
                        &fallback_cwd,
                        &request_cwd,
                        active_tab.viewport,
                    );
                } else {
                    self.open_popup(pipe_message, &request, &fallback_cwd, active_tab.viewport);
                }
            }
            TransientPopupAction::Hide => {
                match select_transient_pane_by_identity(&snapshots, request.spec.identity()) {
                    Some(pane) => {
                        self.displace_other_configured_popups(
                            &request,
                            &snapshots,
                            Some(pane.pane_id),
                            &fallback_cwd,
                        );
                        self.hide_popup(
                            pipe_message,
                            pane.pane_id,
                            request.spec.on_hide.as_ref(),
                            &request_cwd,
                        );
                    }
                    None => self.respond(pipe_message, RESULT_MISSING),
                }
            }
            TransientPopupAction::Close => {
                match select_transient_pane_by_identity(&snapshots, request.spec.identity()) {
                    Some(pane) => {
                        self.displace_other_configured_popups(
                            &request,
                            &snapshots,
                            Some(pane.pane_id),
                            &fallback_cwd,
                        );
                        self.close_popup(
                            pipe_message,
                            pane.pane_id,
                            request.spec.on_close.as_ref(),
                            &request_cwd,
                        );
                    }
                    None => self.respond(pipe_message, RESULT_MISSING),
                }
            }
        }
    }

    fn ensure_ready(&self, pipe_message: &PipeMessage) -> Option<ActiveTab> {
        if !self.permissions_granted {
            self.respond(pipe_message, RESULT_DENIED);
            return None;
        }

        let Some(active_tab) = self.active_tab else {
            self.respond(pipe_message, RESULT_NOT_READY);
            return None;
        };

        Some(active_tab)
    }

    fn launch_fallback_cwd(&self, active_tab_position: usize) -> String {
        self.terminal_panes_by_tab
            .get(&active_tab_position)
            .and_then(|panes| panes.iter().find(|pane| pane.is_focused))
            .and_then(|pane| get_pane_cwd(pane.pane_id).ok())
            .unwrap_or_else(|| self.initial_cwd.clone())
            .display()
            .to_string()
    }

    fn open_popup(
        &mut self,
        pipe_message: &PipeMessage,
        request: &TransientPopupPipeRequest,
        fallback_cwd: &str,
        viewport: PopupViewport,
    ) {
        let Some(launch_plan) = request.launch_plan(fallback_cwd) else {
            self.respond(pipe_message, RESULT_INVALID_PAYLOAD);
            return;
        };
        let launch_cwd = launch_plan.cwd;
        let command_to_run = CommandToRun {
            path: PathBuf::from(launch_plan.command_path),
            args: launch_plan.args,
            cwd: Some(PathBuf::from(&launch_cwd)),
        };
        let pane_id = open_command_pane_floating(
            command_to_run,
            floating_coordinates(self.effective_geometry(launch_plan.geometry), viewport),
            BTreeMap::new(),
        );

        if let Some(pane_id) = pane_id {
            self.popup_launch_cwds.insert(pane_id, launch_cwd);
            self.popup_geometries.insert(pane_id, launch_plan.geometry);
            let pane_title = if request.spec.preserve_terminal_title {
                ""
            } else {
                request.spec.pane_title.trim()
            };
            rename_pane_with_id(pane_id, pane_title);
            self.respond(pipe_message, RESULT_OPENED);
        } else {
            self.respond(pipe_message, RESULT_MISSING);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn show_popup(
        &mut self,
        pipe_message: &PipeMessage,
        request: &TransientPopupPipeRequest,
        pane_id: PaneId,
        is_suppressed: bool,
        fallback_cwd: &str,
        request_cwd: &str,
        viewport: PopupViewport,
    ) {
        if !request.spec.preserve_on_cwd_change {
            let process_cwd = get_pane_cwd(pane_id)
                .ok()
                .map(|cwd| cwd.display().to_string());
            if should_restart_popup_for_cwd(
                is_suppressed || request.cwd.is_some(),
                self.popup_launch_cwds.get(&pane_id).map(String::as_str),
                process_cwd.as_deref(),
                request_cwd,
            ) {
                self.replace_popup(
                    pipe_message,
                    request,
                    pane_id,
                    fallback_cwd,
                    request_cwd,
                    viewport,
                );
                return;
            }
        }

        show_pane_with_id(pane_id, true, true);
        let geometry = request.spec.geometry();
        self.popup_geometries.insert(pane_id, geometry);
        if let Some(coordinates) = floating_coordinates(self.effective_geometry(geometry), viewport)
        {
            change_floating_panes_coordinates(vec![(pane_id, coordinates)]);
        }
        self.respond(pipe_message, RESULT_FOCUSED);
    }

    fn replace_popup(
        &mut self,
        pipe_message: &PipeMessage,
        request: &TransientPopupPipeRequest,
        pane_id: PaneId,
        fallback_cwd: &str,
        request_cwd: &str,
        viewport: PopupViewport,
    ) {
        self.close_popup_pane(pane_id);
        run_command_hook(request.spec.on_close.as_ref(), request_cwd);
        self.open_popup(pipe_message, request, fallback_cwd, viewport);
    }

    fn displace_other_configured_popups(
        &mut self,
        request: &TransientPopupPipeRequest,
        snapshots: &[TransientPaneSnapshot<'_, PaneId>],
        current_pane_id: Option<PaneId>,
        fallback_cwd: &str,
    ) {
        for candidate in self.popup_specs.select_other_configured_panes(
            snapshots,
            request.spec.id.as_str(),
            current_pane_id,
        ) {
            match candidate.toggle_close_behavior {
                TransientPopupToggleCloseBehavior::Close => {
                    self.popup_launch_cwds.remove(&candidate.pane_id);
                    self.popup_geometries.remove(&candidate.pane_id);
                    close_pane_with_id(candidate.pane_id);
                    run_command_hook(candidate.on_close, fallback_cwd);
                }
                TransientPopupToggleCloseBehavior::Hide => {
                    hide_pane_with_id(candidate.pane_id);
                    run_command_hook(candidate.on_hide, fallback_cwd);
                }
            }
        }
    }

    fn close_popup(
        &mut self,
        pipe_message: &PipeMessage,
        pane_id: PaneId,
        on_close: Option<&TransientPopupCommandHook>,
        fallback_cwd: &str,
    ) {
        self.close_popup_pane(pane_id);
        run_command_hook(on_close, fallback_cwd);
        match hide_floating_panes(None) {
            Ok(_) => self.respond(pipe_message, RESULT_CLOSED),
            Err(_) => self.respond(pipe_message, RESULT_CLOSED_FLOATING_CLEANUP_FAILED),
        }
    }

    fn close_popup_pane(&mut self, pane_id: PaneId) {
        self.popup_launch_cwds.remove(&pane_id);
        self.popup_geometries.remove(&pane_id);
        close_pane_with_id(pane_id);
    }

    fn effective_geometry(&self, geometry: TransientPaneGeometry) -> TransientPaneGeometry {
        let active = self.active_tab.is_some_and(|tab| {
            self.left_margin_active_by_tab
                .get(&tab.position)
                .copied()
                .unwrap_or(false)
        });
        effective_geometry(
            geometry,
            self.popup_specs.left_margin_pane_title().is_none() || active,
        )
    }

    fn reflow_visible_popups(&self) {
        let Some(active_tab) = self.active_tab.filter(|tab| tab.floating_panes_visible) else {
            return;
        };
        let Some(panes) = self.terminal_panes_by_tab.get(&active_tab.position) else {
            return;
        };
        let coordinates = panes
            .iter()
            .filter(|pane| pane.is_floating && !pane.is_suppressed)
            .filter_map(|pane| {
                let geometry = self.popup_geometries.get(&pane.pane_id)?;
                floating_coordinates(self.effective_geometry(*geometry), active_tab.viewport)
                    .map(|coordinates| (pane.pane_id, coordinates))
            })
            .collect::<Vec<_>>();
        if !coordinates.is_empty() {
            change_floating_panes_coordinates(coordinates);
        }
    }

    fn hide_popup(
        &self,
        pipe_message: &PipeMessage,
        pane_id: PaneId,
        on_hide: Option<&TransientPopupCommandHook>,
        fallback_cwd: &str,
    ) {
        hide_pane_with_id(pane_id);
        run_command_hook(on_hide, fallback_cwd);
        match hide_floating_panes(None) {
            Ok(_) => self.respond(pipe_message, RESULT_HIDDEN),
            Err(_) => self.respond(pipe_message, RESULT_HIDDEN_FLOATING_CLEANUP_FAILED),
        }
    }

    fn respond(&self, pipe_message: &PipeMessage, result: &str) {
        if let PipeSource::Cli(pipe_id) = &pipe_message.source {
            cli_pipe_output(pipe_id, result);
        }
    }
}

impl TerminalPane {
    fn transient_snapshot(&self) -> TransientPaneSnapshot<'_, PaneId> {
        TransientPaneSnapshot {
            pane_id: self.pane_id,
            title: self.title.as_str(),
            terminal_command: self.terminal_command.as_deref(),
            is_plugin: false,
            exited: self.exited,
            is_floating: self.is_floating,
            is_suppressed: self.is_suppressed,
            is_focused: self.is_focused,
        }
    }
}

fn build_terminal_panes_by_tab(pane_manifest: &PaneManifest) -> HashMap<usize, Vec<TerminalPane>> {
    pane_manifest
        .panes
        .iter()
        .map(|(tab_position, panes)| {
            let terminal_panes = panes
                .iter()
                .filter(|pane| !pane.is_plugin)
                .map(|pane| TerminalPane {
                    pane_id: PaneId::Terminal(pane.id),
                    title: pane.title.clone(),
                    terminal_command: pane.terminal_command.clone(),
                    exited: pane.exited,
                    is_focused: pane.is_focused,
                    is_floating: pane.is_floating,
                    is_suppressed: pane.is_suppressed,
                })
                .collect();
            (*tab_position, terminal_panes)
        })
        .collect()
}

fn build_left_margin_active_by_tab(
    pane_manifest: &PaneManifest,
    pane_title: Option<&str>,
) -> HashMap<usize, bool> {
    pane_manifest
        .panes
        .iter()
        .map(|(tab_position, panes)| {
            let snapshots = panes
                .iter()
                .map(|pane| LeftMarginPaneSnapshot {
                    title: pane.title.as_str(),
                    exited: pane.exited,
                    is_floating: pane.is_floating,
                    is_suppressed: pane.is_suppressed,
                    columns: pane.pane_columns,
                })
                .collect::<Vec<_>>();
            (*tab_position, left_margin_is_active(&snapshots, pane_title))
        })
        .collect()
}

fn run_command_hook(hook: Option<&TransientPopupCommandHook>, fallback_cwd: &str) {
    let Some(hook_plan) = hook.and_then(|hook| hook.launch_plan(fallback_cwd)) else {
        return;
    };

    let argv = hook_plan
        .command
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    run_command_with_env_variables_and_cwd(
        &argv,
        BTreeMap::new(),
        PathBuf::from(hook_plan.cwd),
        BTreeMap::new(),
    );
}
