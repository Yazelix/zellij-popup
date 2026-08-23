use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kdl::{KdlDocument, KdlNode, KdlValue};
use serde::{Deserialize, Serialize};

const RAW_PIPE_NAME: &str = "transient_popup";
const DEFAULT_SPEC_ID: &str = "default";
const DEFAULT_POPUP_CONFIG_KEY: &str = "popup";
const POPUP_DEFAULTS_CONFIG_KEY: &str = "popup_defaults";
const NAMED_POPUPS_CONFIG_KEY: &str = "popups";
const DEFAULT_SIDE_MARGIN: usize = 0;
const DEFAULT_VERTICAL_MARGIN: usize = 0;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransientPopupAction {
    Toggle,
    Open,
    Focus,
    Replace,
    Hide,
    Close,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransientPopupToggleCloseBehavior {
    #[default]
    Close,
    Hide,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransientPopupSpec {
    pub id: String,
    pub pane_title: String,
    #[serde(default)]
    pub preserve_terminal_title: bool,
    #[serde(default)]
    pub command_marker: Option<String>,
    pub command: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub on_close: Option<TransientPopupCommandHook>,
    #[serde(default)]
    pub on_hide: Option<TransientPopupCommandHook>,
    #[serde(default)]
    pub toggle_close_behavior: TransientPopupToggleCloseBehavior,
    #[serde(default)]
    pub side_margin: usize,
    #[serde(default)]
    pub vertical_margin: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransientPopupCommandHook {
    pub command: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransientPopupPipeRequest {
    pub action: TransientPopupAction,
    pub spec: TransientPopupSpec,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfiguredPopupSpecs {
    specs: BTreeMap<String, TransientPopupSpec>,
    invalid_spec_ids: BTreeSet<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfiguredPopupRequest {
    id: String,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PopupMessageRequestError {
    UnknownAction,
    MissingConfiguredSpec(String),
    InvalidConfiguredSpec(String),
    InvalidPayload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransientPaneIdentityView<'a> {
    pub pane_title: &'a str,
    pub command_marker: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransientPaneSnapshot<'a, Id> {
    pub pane_id: Id,
    pub title: &'a str,
    pub terminal_command: Option<&'a str>,
    pub is_plugin: bool,
    pub exited: bool,
    pub is_floating: bool,
    pub is_suppressed: bool,
    pub is_focused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransientPaneState<Id> {
    pub pane_id: Id,
    pub is_focused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransientPaneDisplacementCandidate<'a, Id> {
    pub pane_id: Id,
    pub on_close: Option<&'a TransientPopupCommandHook>,
    pub on_hide: Option<&'a TransientPopupCommandHook>,
    pub toggle_close_behavior: TransientPopupToggleCloseBehavior,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransientTogglePlan<Id> {
    Open,
    Focus(Id),
    ToggleFocused(Id),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransientPaneGeometry {
    pub side_margin: usize,
    pub vertical_margin: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransientPaneLaunchRequest {
    pub command_path: String,
    pub args: Vec<String>,
    pub requested_cwd: Option<String>,
    pub fallback_cwd: String,
    pub geometry: TransientPaneGeometry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransientPaneLaunchPlan {
    pub command_path: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub geometry: TransientPaneGeometry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransientPopupCommandHookPlan {
    pub command: Vec<String>,
    pub cwd: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PopupSpecDraft {
    command: Option<String>,
    args: BTreeMap<usize, String>,
    pane_title: Option<String>,
    preserve_terminal_title: Option<String>,
    command_marker: Option<String>,
    cwd: Option<String>,
    on_close: Option<PopupCommandHookDraft>,
    on_hide: Option<PopupCommandHookDraft>,
    toggle_close_behavior: Option<String>,
    side_margin: Option<String>,
    vertical_margin: Option<String>,
    invalid: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PopupCommandHookDraft {
    command: Option<String>,
    args: BTreeMap<usize, String>,
    cwd: Option<String>,
    invalid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PopupSpecDefaults {
    side_margin: usize,
    vertical_margin: usize,
    on_close: Option<TransientPopupCommandHook>,
    on_hide: Option<TransientPopupCommandHook>,
    invalid: bool,
}

impl Default for PopupSpecDefaults {
    fn default() -> Self {
        Self {
            side_margin: DEFAULT_SIDE_MARGIN,
            vertical_margin: DEFAULT_VERTICAL_MARGIN,
            on_close: None,
            on_hide: None,
            invalid: false,
        }
    }
}

impl TransientPopupAction {
    pub fn from_pipe_name(pipe_name: &str) -> Option<Self> {
        match pipe_name {
            "toggle" => Some(Self::Toggle),
            "open" => Some(Self::Open),
            "focus" => Some(Self::Focus),
            "replace" => Some(Self::Replace),
            "hide" => Some(Self::Hide),
            "close" => Some(Self::Close),
            _ => None,
        }
    }
}

impl ConfiguredPopupSpecs {
    pub fn from_configuration(configuration: &BTreeMap<String, String>) -> Self {
        let mut drafts = BTreeMap::<String, PopupSpecDraft>::new();
        let defaults = configuration
            .get(POPUP_DEFAULTS_CONFIG_KEY)
            .map(|raw_defaults| parse_popup_defaults(raw_defaults))
            .unwrap_or_default();

        if let Some(raw_default_popup) = configuration.get(DEFAULT_POPUP_CONFIG_KEY) {
            parse_popup_fields_into(
                DEFAULT_SPEC_ID,
                raw_default_popup,
                drafts.entry(DEFAULT_SPEC_ID.to_string()).or_default(),
            );
        }

        if let Some(raw_named_popups) = configuration.get(NAMED_POPUPS_CONFIG_KEY) {
            parse_named_popups_into(raw_named_popups, &mut drafts);
        }

        let mut specs = BTreeMap::new();
        let mut invalid_spec_ids = BTreeSet::new();

        for (id, draft) in drafts {
            match build_configured_spec(&id, draft, &defaults) {
                Some(spec) if spec.is_launchable() => {
                    specs.insert(id, spec);
                }
                _ => {
                    invalid_spec_ids.insert(id);
                }
            }
        }

        Self {
            specs,
            invalid_spec_ids,
        }
    }

    pub fn request_from_message(
        &self,
        pipe_name: &str,
        payload: Option<&str>,
    ) -> Result<TransientPopupPipeRequest, PopupMessageRequestError> {
        if pipe_name == RAW_PIPE_NAME {
            return parse_raw_request(payload);
        }

        let Some(action) = TransientPopupAction::from_pipe_name(pipe_name) else {
            return Err(PopupMessageRequestError::UnknownAction);
        };
        let (spec_id, cwd, args) = self.resolve_configured_request(payload)?;

        if self.invalid_spec_ids.contains(&spec_id) {
            return Err(PopupMessageRequestError::InvalidConfiguredSpec(spec_id));
        }
        let Some(spec) = self.specs.get(&spec_id).cloned() else {
            return Err(PopupMessageRequestError::MissingConfiguredSpec(spec_id));
        };

        Ok(TransientPopupPipeRequest {
            action,
            spec,
            args,
            cwd,
        })
    }

    fn resolve_configured_request(
        &self,
        payload: Option<&str>,
    ) -> Result<(String, Option<String>, Vec<String>), PopupMessageRequestError> {
        let Some(payload) = payload.map(str::trim).filter(|value| !value.is_empty()) else {
            let spec_id = if self.specs.contains_key(DEFAULT_SPEC_ID)
                || self.invalid_spec_ids.contains(DEFAULT_SPEC_ID)
            {
                DEFAULT_SPEC_ID.to_string()
            } else if self.specs.len() == 1 && self.invalid_spec_ids.is_empty() {
                self.specs.keys().next().cloned().unwrap_or_default()
            } else {
                return Err(PopupMessageRequestError::InvalidPayload);
            };
            return Ok((spec_id, None, vec![]));
        };
        if !payload.starts_with('{') {
            return Ok((payload.to_string(), None, vec![]));
        }

        let request = serde_json::from_str::<ConfiguredPopupRequest>(payload)
            .map_err(|_| PopupMessageRequestError::InvalidPayload)?;
        let id = request.id.trim();
        let has_cwd = request.cwd.is_some();
        let cwd = request
            .cwd
            .map(|cwd| cwd.trim().to_string())
            .filter(|cwd| !cwd.is_empty());
        if id.is_empty() || has_cwd && cwd.is_none() {
            return Err(PopupMessageRequestError::InvalidPayload);
        }
        Ok((id.to_string(), cwd, request.args))
    }

    pub fn select_other_configured_panes<'a, Id: Copy + PartialEq>(
        &'a self,
        panes: &[TransientPaneSnapshot<'_, Id>],
        current_spec_id: &str,
        current_pane_id: Option<Id>,
    ) -> Vec<TransientPaneDisplacementCandidate<'a, Id>> {
        let mut candidates = Vec::new();

        for (spec_id, spec) in &self.specs {
            if spec_id == current_spec_id {
                continue;
            }

            let Some(pane) = select_visible_transient_pane_by_identity(panes, spec.identity())
            else {
                continue;
            };
            if current_pane_id == Some(pane.pane_id)
                || candidates.iter().any(
                    |candidate: &TransientPaneDisplacementCandidate<'_, Id>| {
                        candidate.pane_id == pane.pane_id
                    },
                )
            {
                continue;
            }

            candidates.push(TransientPaneDisplacementCandidate {
                pane_id: pane.pane_id,
                on_close: spec.on_close.as_ref(),
                on_hide: spec.on_hide.as_ref(),
                toggle_close_behavior: spec.toggle_close_behavior,
            });
        }

        candidates
    }
}

impl TransientPopupSpec {
    pub fn identity(&self) -> TransientPaneIdentityView<'_> {
        TransientPaneIdentityView {
            pane_title: self.pane_title.as_str(),
            command_marker: self
                .command_marker
                .as_deref()
                .map(str::trim)
                .filter(|marker| !marker.is_empty()),
        }
    }

    pub fn geometry(&self) -> TransientPaneGeometry {
        TransientPaneGeometry {
            side_margin: self.side_margin,
            vertical_margin: self.vertical_margin,
        }
    }

    fn is_launchable(&self) -> bool {
        if self.id.trim().is_empty() || self.pane_title.trim().is_empty() {
            return false;
        }

        if self
            .command_marker
            .as_deref()
            .is_some_and(|marker| marker.trim().is_empty())
        {
            return false;
        }

        self.command
            .first()
            .is_some_and(|command_path| !command_path.trim().is_empty())
            && self.command.iter().all(|arg| !arg.trim().is_empty())
            && self
                .cwd
                .as_deref()
                .map(str::trim)
                .is_none_or(|cwd| !cwd.is_empty())
            && self
                .on_close
                .as_ref()
                .is_none_or(TransientPopupCommandHook::is_launchable)
            && self
                .on_hide
                .as_ref()
                .is_none_or(TransientPopupCommandHook::is_launchable)
    }
}

impl TransientPopupCommandHook {
    pub fn launch_plan(&self, fallback_cwd: &str) -> Option<TransientPopupCommandHookPlan> {
        if !self.is_launchable() {
            return None;
        }

        Some(TransientPopupCommandHookPlan {
            command: self.command.clone(),
            cwd: resolve_launch_cwd(
                self.cwd
                    .as_deref()
                    .map(str::trim)
                    .filter(|cwd| !cwd.is_empty()),
                fallback_cwd.trim(),
            )?,
        })
    }

    fn is_launchable(&self) -> bool {
        self.command
            .first()
            .is_some_and(|command_path| !command_path.trim().is_empty())
            && self.command.iter().all(|arg| !arg.trim().is_empty())
            && self
                .cwd
                .as_deref()
                .map(str::trim)
                .is_none_or(|cwd| !cwd.is_empty())
    }
}

impl TransientPopupPipeRequest {
    pub fn is_launchable_spec(&self) -> bool {
        self.spec.is_launchable()
    }

    pub fn launch_plan(&self, fallback_cwd: &str) -> Option<TransientPaneLaunchPlan> {
        if self.spec.id.trim().is_empty() || self.spec.pane_title.trim().is_empty() {
            return None;
        }

        let command_path = self.spec.command.first()?.clone();
        let mut args = self
            .spec
            .command
            .iter()
            .skip(1)
            .cloned()
            .collect::<Vec<_>>();
        args.extend(self.args.iter().cloned());

        resolve_transient_launch_plan(TransientPaneLaunchRequest {
            command_path,
            args,
            requested_cwd: self.cwd.clone().or_else(|| self.spec.cwd.clone()),
            fallback_cwd: fallback_cwd.to_string(),
            geometry: self.spec.geometry(),
        })
    }
}

pub fn resolve_transient_launch_plan(
    request: TransientPaneLaunchRequest,
) -> Option<TransientPaneLaunchPlan> {
    let command_path = request.command_path.trim();
    if command_path.is_empty() {
        return None;
    }
    let requested_cwd = request
        .requested_cwd
        .as_deref()
        .map(str::trim)
        .filter(|cwd| !cwd.is_empty());
    let cwd = resolve_launch_cwd(requested_cwd, request.fallback_cwd.trim())?;
    if cwd.is_empty() {
        return None;
    }

    Some(TransientPaneLaunchPlan {
        command_path: command_path.to_string(),
        args: request.args,
        cwd,
        geometry: request.geometry,
    })
}

fn resolve_launch_cwd(requested_cwd: Option<&str>, fallback_cwd: &str) -> Option<String> {
    match requested_cwd {
        Some(cwd) if Path::new(cwd).is_absolute() => Some(cwd.to_string()),
        Some(cwd) => {
            if fallback_cwd.is_empty() {
                return None;
            }

            Some(Path::new(fallback_cwd).join(cwd).display().to_string())
        }
        None => (!fallback_cwd.is_empty()).then(|| fallback_cwd.to_string()),
    }
}

pub fn select_transient_pane_by_identity<Id: Copy>(
    panes: &[TransientPaneSnapshot<'_, Id>],
    identity: TransientPaneIdentityView<'_>,
) -> Option<TransientPaneState<Id>> {
    panes
        .iter()
        .filter(|pane| {
            (pane.is_floating || pane.is_suppressed) && pane_matches_identity(pane, identity)
        })
        .max_by_key(|pane| (pane.is_focused, !pane.is_suppressed))
        .map(|pane| TransientPaneState {
            pane_id: pane.pane_id,
            is_focused: pane.is_focused,
        })
}

fn select_visible_transient_pane_by_identity<Id: Copy>(
    panes: &[TransientPaneSnapshot<'_, Id>],
    identity: TransientPaneIdentityView<'_>,
) -> Option<TransientPaneState<Id>> {
    panes
        .iter()
        .filter(|pane| {
            pane.is_floating && !pane.is_suppressed && pane_matches_identity(pane, identity)
        })
        .max_by_key(|pane| pane.is_focused)
        .map(|pane| TransientPaneState {
            pane_id: pane.pane_id,
            is_focused: pane.is_focused,
        })
}

fn pane_matches_identity<Id>(
    pane: &TransientPaneSnapshot<'_, Id>,
    identity: TransientPaneIdentityView<'_>,
) -> bool {
    !pane.is_plugin
        && (pane.title.trim() == identity.pane_title
            || identity.command_marker.is_some_and(|command_marker| {
                pane.terminal_command
                    .map(|command| command.contains(command_marker))
                    .unwrap_or(false)
            }))
}

pub fn resolve_transient_toggle_plan_by_identity<Id: Copy>(
    panes: &[TransientPaneSnapshot<'_, Id>],
    identity: TransientPaneIdentityView<'_>,
    floating_panes_visible: bool,
) -> TransientTogglePlan<Id> {
    match select_transient_pane_by_identity(panes, identity) {
        Some(pane) if pane.is_focused && floating_panes_visible => {
            TransientTogglePlan::ToggleFocused(pane.pane_id)
        }
        Some(pane) => TransientTogglePlan::Focus(pane.pane_id),
        None => TransientTogglePlan::Open,
    }
}

/// Popups whose cwd must match should restart when their remembered launch cwd differs
/// from the effective cwd of a fresh launch. Without launch state, a live process below
/// the effective cwd remains reusable.
pub fn should_restart_popup_for_cwd(
    cwd_must_match: bool,
    launch_cwd: Option<&str>,
    process_cwd: Option<&str>,
    effective_cwd: &str,
) -> bool {
    if !cwd_must_match {
        return false;
    }
    let effective_cwd = effective_cwd.trim();
    if effective_cwd.is_empty() {
        return false;
    }
    let effective_cwd = Path::new(effective_cwd);
    if let Some(launch_cwd) = launch_cwd.map(str::trim).filter(|cwd| !cwd.is_empty()) {
        return Path::new(launch_cwd) != effective_cwd;
    }
    process_cwd
        .map(str::trim)
        .filter(|cwd| !cwd.is_empty())
        .is_some_and(|cwd| !Path::new(cwd).starts_with(effective_cwd))
}

fn parse_raw_request(
    payload: Option<&str>,
) -> Result<TransientPopupPipeRequest, PopupMessageRequestError> {
    let Some(payload) = payload else {
        return Err(PopupMessageRequestError::InvalidPayload);
    };
    match serde_json::from_str::<TransientPopupPipeRequest>(payload) {
        Ok(request) if request.is_launchable_spec() => Ok(request),
        _ => Err(PopupMessageRequestError::InvalidPayload),
    }
}

fn build_configured_spec(
    id: &str,
    draft: PopupSpecDraft,
    defaults: &PopupSpecDefaults,
) -> Option<TransientPopupSpec> {
    if id.trim().is_empty() || draft.invalid || defaults.invalid {
        return None;
    }

    let command_path = trim_required(draft.command)?;
    let mut command = vec![command_path.clone()];
    for arg in draft.args.into_values() {
        command.push(trim_required(Some(arg))?);
    }

    Some(TransientPopupSpec {
        id: id.trim().to_string(),
        pane_title: trim_optional(draft.pane_title).unwrap_or_else(|| format!("{id}_popup")),
        preserve_terminal_title: parse_bool(draft.preserve_terminal_title, false)?,
        command_marker: trim_optional(draft.command_marker).or(Some(command_path)),
        command,
        cwd: trim_optional(draft.cwd),
        on_close: match draft.on_close {
            Some(hook) => Some(build_configured_hook(hook)?),
            None => defaults.on_close.clone(),
        },
        on_hide: match draft.on_hide {
            Some(hook) => Some(build_configured_hook(hook)?),
            None => defaults.on_hide.clone(),
        },
        toggle_close_behavior: parse_toggle_close_behavior(draft.toggle_close_behavior)?,
        side_margin: parse_margin(draft.side_margin, defaults.side_margin)?,
        vertical_margin: parse_margin(draft.vertical_margin, defaults.vertical_margin)?,
    })
}

fn build_configured_hook(draft: PopupCommandHookDraft) -> Option<TransientPopupCommandHook> {
    if draft.invalid {
        return None;
    }

    let command_path = trim_required(draft.command)?;
    let mut command = vec![command_path];
    for arg in draft.args.into_values() {
        command.push(trim_required(Some(arg))?);
    }

    Some(TransientPopupCommandHook {
        command,
        cwd: trim_optional(draft.cwd),
    })
}

fn parse_named_popups_into(raw: &str, drafts: &mut BTreeMap<String, PopupSpecDraft>) {
    let Ok(document) = raw.parse::<KdlDocument>() else {
        drafts
            .entry(NAMED_POPUPS_CONFIG_KEY.to_string())
            .or_default()
            .invalid = true;
        return;
    };

    for popup_node in document.nodes() {
        let id = popup_node.name().value().trim();
        let draft = drafts.entry(id.to_string()).or_default();
        if id.is_empty() {
            draft.invalid = true;
            continue;
        }

        let Some(children) = popup_node.children() else {
            draft.invalid = true;
            continue;
        };

        parse_popup_fields_document_into(children, draft);
    }
}

fn parse_popup_fields_into(id: &str, raw: &str, draft: &mut PopupSpecDraft) {
    if id.trim().is_empty() {
        draft.invalid = true;
        return;
    }

    let Ok(document) = raw.parse::<KdlDocument>() else {
        draft.invalid = true;
        return;
    };

    parse_popup_fields_document_into(&document, draft);
}

fn parse_popup_defaults(raw: &str) -> PopupSpecDefaults {
    let mut defaults = PopupSpecDefaults::default();
    let mut on_close = None;
    let mut on_hide = None;
    let Ok(document) = raw.parse::<KdlDocument>() else {
        defaults.invalid = true;
        return defaults;
    };

    for field_node in document.nodes() {
        let field_name = field_node.name().value();
        match field_name {
            "on_close" => {
                parse_hook_node_into(field_node, on_close.get_or_insert_with(Default::default));
                continue;
            }
            "on_hide" => {
                parse_hook_node_into(field_node, on_hide.get_or_insert_with(Default::default));
                continue;
            }
            _ => {}
        }

        let Some(value) = popup_field_value(field_node) else {
            defaults.invalid = true;
            continue;
        };
        match field_name {
            "side_margin" => match parse_margin(Some(value), DEFAULT_SIDE_MARGIN) {
                Some(side_margin) => defaults.side_margin = side_margin,
                None => defaults.invalid = true,
            },
            "vertical_margin" => match parse_margin(Some(value), DEFAULT_VERTICAL_MARGIN) {
                Some(vertical_margin) => defaults.vertical_margin = vertical_margin,
                None => defaults.invalid = true,
            },
            _ => defaults.invalid = true,
        }
    }

    if let Some(hook) = on_close {
        match build_configured_hook(hook) {
            Some(hook) => defaults.on_close = Some(hook),
            None => defaults.invalid = true,
        }
    }
    if let Some(hook) = on_hide {
        match build_configured_hook(hook) {
            Some(hook) => defaults.on_hide = Some(hook),
            None => defaults.invalid = true,
        }
    }

    defaults
}

fn parse_popup_fields_document_into(document: &KdlDocument, draft: &mut PopupSpecDraft) {
    for field_node in document.nodes() {
        let field_name = field_node.name().value();
        if field_name == "on_close" {
            parse_hook_node_into(
                field_node,
                draft.on_close.get_or_insert_with(Default::default),
            );
            continue;
        }
        if field_name == "on_hide" {
            parse_hook_node_into(
                field_node,
                draft.on_hide.get_or_insert_with(Default::default),
            );
            continue;
        }

        let Some(field) = popup_config_field(field_name) else {
            draft.invalid = true;
            continue;
        };
        let Some(value) = popup_field_value(field_node) else {
            draft.invalid = true;
            continue;
        };

        apply_config_field(draft, field, value);
    }
}

fn parse_hook_node_into(field_node: &KdlNode, draft: &mut PopupCommandHookDraft) {
    let Some(children) = field_node.children() else {
        draft.invalid = true;
        return;
    };

    for hook_node in children.nodes() {
        let field_name = hook_node.name().value();
        let Some(field) = hook_config_field(field_name) else {
            draft.invalid = true;
            continue;
        };
        let Some(value) = popup_field_value(hook_node) else {
            draft.invalid = true;
            continue;
        };

        apply_hook_config_field(draft, field, value);
    }
}

fn popup_field_value(field_node: &KdlNode) -> Option<String> {
    field_node
        .entries()
        .iter()
        .find(|entry| entry.name().is_none())
        .and_then(|entry| kdl_value_to_string(entry.value()))
}

fn kdl_value_to_string(value: &KdlValue) -> Option<String> {
    value
        .as_string()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_i64().map(|value| value.to_string()))
        .or_else(|| value.as_bool().map(|value| value.to_string()))
}

fn apply_config_field(draft: &mut PopupSpecDraft, field: PopupConfigField, value: String) {
    match field {
        PopupConfigField::Command => draft.command = Some(value),
        PopupConfigField::PaneTitle => draft.pane_title = Some(value),
        PopupConfigField::PreserveTerminalTitle => draft.preserve_terminal_title = Some(value),
        PopupConfigField::CommandMarker => draft.command_marker = Some(value),
        PopupConfigField::Cwd => draft.cwd = Some(value),
        PopupConfigField::ToggleCloseBehavior => draft.toggle_close_behavior = Some(value),
        PopupConfigField::SideMargin => draft.side_margin = Some(value),
        PopupConfigField::VerticalMargin => draft.vertical_margin = Some(value),
        PopupConfigField::Arg(index) => {
            if index == 0 {
                draft.invalid = true;
            } else {
                draft.args.insert(index, value);
            }
        }
    }
}

fn apply_hook_config_field(
    draft: &mut PopupCommandHookDraft,
    field: PopupCommandHookField,
    value: String,
) {
    match field {
        PopupCommandHookField::Command => draft.command = Some(value),
        PopupCommandHookField::Cwd => draft.cwd = Some(value),
        PopupCommandHookField::Arg(index) => {
            if index == 0 {
                draft.invalid = true;
            } else {
                draft.args.insert(index, value);
            }
        }
    }
}

fn trim_required(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_toggle_close_behavior(value: Option<String>) -> Option<TransientPopupToggleCloseBehavior> {
    match trim_optional(value).as_deref() {
        None | Some("close") => Some(TransientPopupToggleCloseBehavior::Close),
        Some("hide") => Some(TransientPopupToggleCloseBehavior::Hide),
        Some(_) => None,
    }
}

fn parse_bool(value: Option<String>, default: bool) -> Option<bool> {
    match trim_optional(value).as_deref() {
        None => Some(default),
        Some("true") => Some(true),
        Some("false") => Some(false),
        Some(_) => None,
    }
}

fn parse_margin(value: Option<String>, default: usize) -> Option<usize> {
    match value {
        Some(value) => value.trim().parse::<usize>().ok(),
        None => Some(default),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupConfigField {
    Command,
    PaneTitle,
    PreserveTerminalTitle,
    CommandMarker,
    Cwd,
    ToggleCloseBehavior,
    SideMargin,
    VerticalMargin,
    Arg(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupCommandHookField {
    Command,
    Cwd,
    Arg(usize),
}

fn popup_config_field(key: &str) -> Option<PopupConfigField> {
    if let Some(index) = key.strip_prefix("arg_") {
        index.parse::<usize>().ok().map(PopupConfigField::Arg)
    } else if key == "command" {
        Some(PopupConfigField::Command)
    } else if key == "pane_title" {
        Some(PopupConfigField::PaneTitle)
    } else if key == "preserve_terminal_title" {
        Some(PopupConfigField::PreserveTerminalTitle)
    } else if key == "command_marker" {
        Some(PopupConfigField::CommandMarker)
    } else if key == "cwd" {
        Some(PopupConfigField::Cwd)
    } else if key == "toggle_close_behavior" {
        Some(PopupConfigField::ToggleCloseBehavior)
    } else if key == "side_margin" {
        Some(PopupConfigField::SideMargin)
    } else if key == "vertical_margin" {
        Some(PopupConfigField::VerticalMargin)
    } else {
        None
    }
}

fn hook_config_field(key: &str) -> Option<PopupCommandHookField> {
    if let Some(index) = key.strip_prefix("arg_") {
        index.parse::<usize>().ok().map(PopupCommandHookField::Arg)
    } else if key == "command" {
        Some(PopupCommandHookField::Command)
    } else if key == "cwd" {
        Some(PopupCommandHookField::Cwd)
    } else {
        None
    }
}

// Test lane: default
#[cfg(test)]
mod tests {
    use super::{
        resolve_transient_toggle_plan_by_identity, should_restart_popup_for_cwd,
        ConfiguredPopupSpecs, PopupMessageRequestError, TransientPaneDisplacementCandidate,
        TransientPaneGeometry, TransientPaneSnapshot, TransientPaneState, TransientPopupAction,
        TransientPopupToggleCloseBehavior, TransientTogglePlan,
    };
    use std::collections::BTreeMap;

    fn config(values: &[(&str, &str)]) -> BTreeMap<String, String> {
        values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    fn transient_pane<'a>(
        pane_id: i32,
        title: &'a str,
        terminal_command: Option<&'a str>,
        is_focused: bool,
    ) -> TransientPaneSnapshot<'a, i32> {
        TransientPaneSnapshot {
            pane_id,
            title,
            terminal_command,
            is_plugin: false,
            exited: false,
            is_floating: true,
            is_suppressed: false,
            is_focused,
        }
    }

    fn suppressed_transient_pane<'a>(
        pane_id: i32,
        title: &'a str,
        terminal_command: Option<&'a str>,
    ) -> TransientPaneSnapshot<'a, i32> {
        TransientPaneSnapshot {
            pane_id,
            title,
            terminal_command,
            is_plugin: false,
            exited: false,
            is_floating: false,
            is_suppressed: true,
            is_focused: false,
        }
    }

    fn keep_alive_and_gitui_specs() -> ConfiguredPopupSpecs {
        ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                process_monitor {
                    command "yzx"
                    arg_1 "popup_run"
                    arg_2 "btm"
                    toggle_close_behavior "hide"
                }
                gitui {
                    command "gitui"
                }
            "#,
        )]))
    }

    #[test]
    fn configured_spec_builds_kdl_native_request() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popup",
            r#"
                    command "gitui"
                    arg_1 "--watch"
                    pane_title "gitui_popup"
                    preserve_terminal_title true
                    cwd "."
                    side_margin 2
                    vertical_margin 1
                "#,
        )]));

        let request = specs
            .request_from_message("toggle", None)
            .expect("configured request");

        assert_eq!(request.action, TransientPopupAction::Toggle);
        assert_eq!(request.spec.id, "default");
        assert_eq!(request.spec.command, vec!["gitui", "--watch"]);
        assert_eq!(request.spec.command_marker.as_deref(), Some("gitui"));
        assert!(request.spec.preserve_terminal_title);
        assert_eq!(
            request.spec.toggle_close_behavior,
            TransientPopupToggleCloseBehavior::Close
        );
        assert_eq!(
            request.spec.geometry(),
            TransientPaneGeometry {
                side_margin: 2,
                vertical_margin: 1,
            }
        );
        assert_eq!(
            request.launch_plan("/fallback").expect("launch plan").cwd,
            "/fallback/."
        );

        let replace = specs
            .request_from_message("replace", None)
            .expect("configured replace request");
        assert_eq!(replace.action, TransientPopupAction::Replace);

        let hide = specs
            .request_from_message("hide", None)
            .expect("configured hide request");
        assert_eq!(hide.action, TransientPopupAction::Hide);
    }

    #[test]
    // Defends: plugin-level geometry defaults apply to configured popup specs.
    fn popup_defaults_apply_margins_to_named_popups() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[
            (
                "popup_defaults",
                r#"
                    side_margin 1
                    vertical_margin 0
                "#,
            ),
            (
                "popups",
                r#"
                    gitui {
                        command "gitui"
                    }
                    lazygit {
                        command "lazygit"
                    }
                "#,
            ),
        ]));

        for popup_id in ["gitui", "lazygit"] {
            let request = specs
                .request_from_message("toggle", Some(popup_id))
                .expect("named configured request");

            assert_eq!(
                request.spec.geometry(),
                TransientPaneGeometry {
                    side_margin: 1,
                    vertical_margin: 0,
                }
            );
        }
    }

    #[test]
    // Defends: per-popup geometry fields override plugin-level defaults.
    fn popup_defaults_allow_per_popup_margin_overrides() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[
            (
                "popup_defaults",
                r#"
                    side_margin 1
                    vertical_margin 0
                "#,
            ),
            (
                "popups",
                r#"
                    gitui {
                        command "gitui"
                        side_margin 3
                        vertical_margin 2
                    }
                "#,
            ),
        ]));

        let request = specs
            .request_from_message("toggle", Some("gitui"))
            .expect("named configured request");

        assert_eq!(
            request.spec.geometry(),
            TransientPaneGeometry {
                side_margin: 3,
                vertical_margin: 2,
            }
        );
    }

    #[test]
    // Defends: plugin-level hook defaults apply to configured popup specs.
    fn popup_defaults_apply_hooks_to_named_popups() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[
            (
                "popup_defaults",
                r#"
                    on_close {
                        command "hook"
                        arg_1 "close"
                    }
                    on_hide {
                        command "hook"
                        arg_1 "hide"
                    }
                "#,
            ),
            (
                "popups",
                r#"
                    gitui {
                        command "gitui"
                    }
                "#,
            ),
        ]));

        let request = specs
            .request_from_message("toggle", Some("gitui"))
            .expect("named configured request");
        let close_plan = request
            .spec
            .on_close
            .as_ref()
            .and_then(|hook| hook.launch_plan("/repo"))
            .expect("close hook plan");
        let hide_plan = request
            .spec
            .on_hide
            .as_ref()
            .and_then(|hook| hook.launch_plan("/repo"))
            .expect("hide hook plan");

        assert_eq!(close_plan.command, vec!["hook", "close"]);
        assert_eq!(hide_plan.command, vec!["hook", "hide"]);
    }

    #[test]
    // Defends: per-popup hooks override the corresponding plugin-level hook default.
    fn popup_defaults_allow_per_popup_hook_overrides() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[
            (
                "popup_defaults",
                r#"
                    on_hide {
                        command "hook"
                        arg_1 "default"
                    }
                "#,
            ),
            (
                "popups",
                r#"
                    gitui {
                        command "gitui"
                        on_hide {
                            command "hook"
                            arg_1 "gitui"
                            cwd "."
                        }
                    }
                "#,
            ),
        ]));

        let request = specs
            .request_from_message("toggle", Some("gitui"))
            .expect("named configured request");
        let hook_plan = request
            .spec
            .on_hide
            .as_ref()
            .and_then(|hook| hook.launch_plan("/repo"))
            .expect("hide hook plan");

        assert_eq!(hook_plan.command, vec!["hook", "gitui"]);
        assert_eq!(hook_plan.cwd, "/repo/.");
    }

    #[test]
    fn relative_configured_cwd_resolves_against_focused_fallback() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popup",
            r#"
                    command "gitui"
                    cwd "tools"
                "#,
        )]));

        let request = specs
            .request_from_message("toggle", None)
            .expect("configured request");

        assert_eq!(
            request.launch_plan("/repo").expect("launch plan").cwd,
            "/repo/tools"
        );
    }

    #[test]
    fn configured_spec_without_cwd_uses_focused_fallback() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popup",
            r#"
                    command "gitui"
                "#,
        )]));

        let request = specs
            .request_from_message("toggle", None)
            .expect("configured request");

        assert_eq!(
            request.launch_plan("/repo").expect("launch plan").cwd,
            "/repo"
        );
    }

    #[test]
    fn configured_request_adds_runtime_args_and_cwd_without_copying_the_spec() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                agent {
                    command "codex"
                    pane_title "agent_popup"
                    toggle_close_behavior "hide"
                }
            "#,
        )]));

        let request = specs
            .request_from_message(
                "replace",
                Some(r#"{"id":"agent","cwd":"/repo","args":["/repo/file with spaces"]}"#),
            )
            .expect("configured request with runtime values");

        assert_eq!(request.action, TransientPopupAction::Replace);
        assert_eq!(request.spec.id, "agent");
        assert_eq!(request.spec.command, vec!["codex"]);
        let launch_plan = request.launch_plan("/repo/docs").unwrap();
        assert_eq!(launch_plan.cwd, "/repo");
        assert_eq!(launch_plan.args, vec!["/repo/file with spaces"]);
        assert!(!should_restart_popup_for_cwd(
            true,
            Some("/repo"),
            None,
            &launch_plan.cwd,
        ));
        assert!(should_restart_popup_for_cwd(
            true,
            Some("/old"),
            None,
            &launch_plan.cwd,
        ));
    }

    #[test]
    fn absolute_configured_cwd_is_preserved() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popup",
            r#"
                    command "gitui"
                    cwd "/tmp/repo"
                "#,
        )]));

        let request = specs
            .request_from_message("toggle", None)
            .expect("configured request");

        assert_eq!(
            request.launch_plan("/repo").expect("launch plan").cwd,
            "/tmp/repo"
        );
    }

    #[test]
    fn nested_popups_support_multiple_named_popups() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                gitui {
                    command "gitui"
                    pane_title "gitui_popup"
                }
                lazygit {
                    command "lazygit"
                    pane_title "lazygit_popup"
                }
            "#,
        )]));

        let request = specs
            .request_from_message("toggle", Some("lazygit"))
            .expect("named configured request");

        assert_eq!(request.spec.id, "lazygit");
        assert_eq!(request.spec.command, vec!["lazygit"]);
        assert_eq!(request.spec.pane_title, "lazygit_popup");
    }

    #[test]
    fn configured_spec_parses_toggle_close_behavior() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                btm {
                    command "btm"
                    toggle_close_behavior "hide"
                }
            "#,
        )]));

        let request = specs
            .request_from_message("toggle", Some("btm"))
            .expect("named configured request");

        assert_eq!(
            request.spec.toggle_close_behavior,
            TransientPopupToggleCloseBehavior::Hide
        );
    }

    #[test]
    fn configured_spec_parses_on_close_hook() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                lazygit {
                    command "lazygit"
                    on_close {
                        command "hook"
                        arg_1 "close"
                        cwd "."
                    }
                }
            "#,
        )]));

        let request = specs
            .request_from_message("toggle", Some("lazygit"))
            .expect("named configured request");
        let hook_plan = request
            .spec
            .on_close
            .as_ref()
            .and_then(|hook| hook.launch_plan("/repo"))
            .expect("hook plan");

        assert_eq!(hook_plan.command, vec!["hook", "close"]);
        assert_eq!(hook_plan.cwd, "/repo/.");
    }

    #[test]
    // Defends: per-popup on_hide uses the same argv/cwd hook shape as on_close.
    fn configured_spec_parses_on_hide_hook() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                lazygit {
                    command "lazygit"
                    on_hide {
                        command "hook"
                        arg_1 "hide"
                        cwd "."
                    }
                }
            "#,
        )]));

        let request = specs
            .request_from_message("toggle", Some("lazygit"))
            .expect("named configured request");
        let hook_plan = request
            .spec
            .on_hide
            .as_ref()
            .and_then(|hook| hook.launch_plan("/repo"))
            .expect("hook plan");

        assert_eq!(hook_plan.command, vec!["hook", "hide"]);
        assert_eq!(hook_plan.cwd, "/repo/.");
    }

    #[test]
    fn configured_spec_rejects_invalid_on_close_hook() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                lazygit {
                    command "lazygit"
                    on_close {
                        arg_1 "close"
                    }
                }
            "#,
        )]));

        assert_eq!(
            specs.request_from_message("toggle", Some("lazygit")),
            Err(PopupMessageRequestError::InvalidConfiguredSpec(
                "lazygit".into()
            ))
        );
    }

    #[test]
    // Defends: invalid per-popup on_hide hooks make the configured popup invalid.
    fn configured_spec_rejects_invalid_on_hide_hook() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                lazygit {
                    command "lazygit"
                    on_hide {
                        arg_1 "hide"
                    }
                }
            "#,
        )]));

        assert_eq!(
            specs.request_from_message("toggle", Some("lazygit")),
            Err(PopupMessageRequestError::InvalidConfiguredSpec(
                "lazygit".into()
            ))
        );
    }

    #[test]
    fn configured_spec_rejects_invalid_toggle_close_behavior() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                btm {
                    command "btm"
                    toggle_close_behavior "keep_alive"
                }
            "#,
        )]));

        assert_eq!(
            specs.request_from_message("toggle", Some("btm")),
            Err(PopupMessageRequestError::InvalidConfiguredSpec(
                "btm".into()
            ))
        );
    }

    #[test]
    fn configured_spec_defaults_to_single_spec_without_payload() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                lazygit {
                    command "lazygit"
                }
            "#,
        )]));

        let request = specs
            .request_from_message("open", None)
            .expect("single configured request");

        assert_eq!(request.action, TransientPopupAction::Open);
        assert_eq!(request.spec.id, "lazygit");
        assert_eq!(request.spec.pane_title, "lazygit_popup");
        assert_eq!(request.spec.side_margin, 0);
        assert_eq!(request.spec.vertical_margin, 0);
    }

    #[test]
    fn configured_spec_rejects_removed_percentage_fields() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                gitui {
                    command "gitui"
                    width_percent 90
                }
            "#,
        )]));

        assert_eq!(
            specs.request_from_message("toggle", Some("gitui")),
            Err(PopupMessageRequestError::InvalidConfiguredSpec(
                "gitui".into()
            ))
        );
    }

    #[test]
    // Defends: invalid plugin-level defaults fail visibly instead of being ignored.
    fn popup_defaults_return_invalid_config_for_bad_margin() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[
            (
                "popup_defaults",
                r#"
                    side_margin "wide"
                "#,
            ),
            (
                "popups",
                r#"
                    gitui {
                        command "gitui"
                    }
                "#,
            ),
        ]));

        assert_eq!(
            specs.request_from_message("toggle", Some("gitui")),
            Err(PopupMessageRequestError::InvalidConfiguredSpec(
                "gitui".into()
            ))
        );
    }

    #[test]
    // Defends: invalid plugin-level hook defaults fail visibly instead of being ignored.
    fn popup_defaults_return_invalid_config_for_bad_hook() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[
            (
                "popup_defaults",
                r#"
                    on_hide {
                        arg_1 "hide"
                    }
                "#,
            ),
            (
                "popups",
                r#"
                    gitui {
                        command "gitui"
                    }
                "#,
            ),
        ]));

        assert_eq!(
            specs.request_from_message("toggle", Some("gitui")),
            Err(PopupMessageRequestError::InvalidConfiguredSpec(
                "gitui".into()
            ))
        );
    }

    #[test]
    fn raw_json_request_still_works_for_generated_callers() {
        let specs = ConfiguredPopupSpecs::default();
        let payload = r#"{
            "action": "close",
            "spec": {
                "id": "gitui",
                "pane_title": "gitui_popup",
                "command_marker": "gitui",
                "command": ["gitui"],
                "cwd": ".",
                "on_hide": {
                    "command": ["hook", "hide"]
                },
                "toggle_close_behavior": "hide"
            }
        }"#;

        let request = specs
            .request_from_message("transient_popup", Some(payload))
            .expect("raw request");

        assert_eq!(request.action, TransientPopupAction::Close);
        assert_eq!(request.spec.id, "gitui");
        assert_eq!(
            request.spec.toggle_close_behavior,
            TransientPopupToggleCloseBehavior::Hide
        );
        assert_eq!(
            request
                .spec
                .on_hide
                .as_ref()
                .and_then(|hook| hook.launch_plan("/repo"))
                .expect("hide hook plan")
                .command,
            vec!["hook", "hide"]
        );
    }

    #[test]
    fn raw_json_request_rejects_removed_percentage_fields() {
        let specs = ConfiguredPopupSpecs::default();
        let payload = r#"{
            "action": "open",
            "spec": {
                "id": "gitui",
                "pane_title": "gitui_popup",
                "command": ["gitui"],
                "width_percent": 90
            }
        }"#;

        assert_eq!(
            specs.request_from_message("transient_popup", Some(payload)),
            Err(PopupMessageRequestError::InvalidPayload)
        );
    }

    #[test]
    fn toggle_plan_respects_tab_wide_floating_visibility() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                gitui {
                    command "gitui"
                }
            "#,
        )]));
        let request = specs
            .request_from_message("toggle", Some("gitui"))
            .expect("request");
        let focused = [transient_pane(11, "other", Some("gitui"), true)];
        let unfocused = [transient_pane(12, "other", Some("gitui"), false)];
        let suppressed = [suppressed_transient_pane(13, "other", Some("gitui"))];
        let focused_exited = [TransientPaneSnapshot {
            pane_id: 14,
            exited: true,
            ..focused[0]
        }];
        let unfocused_exited = [TransientPaneSnapshot {
            pane_id: 15,
            exited: true,
            ..unfocused[0]
        }];

        assert_eq!(
            resolve_transient_toggle_plan_by_identity(&focused, request.spec.identity(), true),
            TransientTogglePlan::ToggleFocused(11)
        );
        assert_eq!(
            resolve_transient_toggle_plan_by_identity(&focused, request.spec.identity(), false),
            TransientTogglePlan::Focus(11),
            "a globally hidden floating layer must override the stale pane focus bit"
        );
        assert_eq!(
            resolve_transient_toggle_plan_by_identity(&unfocused, request.spec.identity(), true),
            TransientTogglePlan::Focus(12)
        );
        assert_eq!(
            resolve_transient_toggle_plan_by_identity(&suppressed, request.spec.identity(), false),
            TransientTogglePlan::Focus(13)
        );
        assert_eq!(
            resolve_transient_toggle_plan_by_identity(
                &focused_exited,
                request.spec.identity(),
                true
            ),
            TransientTogglePlan::ToggleFocused(14)
        );
        assert_eq!(
            resolve_transient_toggle_plan_by_identity(
                &unfocused_exited,
                request.spec.identity(),
                true
            ),
            TransientTogglePlan::Focus(15)
        );
        assert_eq!(
            resolve_transient_toggle_plan_by_identity::<i32>(&[], request.spec.identity(), false),
            TransientTogglePlan::Open
        );
        assert_eq!(
            super::select_transient_pane_by_identity(&focused, request.spec.identity()),
            Some(TransientPaneState {
                pane_id: 11,
                is_focused: true,
            })
        );
    }

    #[test]
    fn keep_alive_popup_cwd_uses_launch_state_before_process_cwd() {
        assert!(
            !should_restart_popup_for_cwd(true, Some("/repo"), Some("/repo/subdir"), "/repo"),
            "application navigation does not change the popup launch cwd"
        );
        assert!(
            should_restart_popup_for_cwd(true, Some("/old"), Some("/old/subdir"), "/repo"),
            "launch cwd mismatch restarts the hidden keep-alive pane"
        );
        assert!(
            !should_restart_popup_for_cwd(true, None, Some("/repo/subdir"), "/repo"),
            "missing launch state reuses navigation below the requested root"
        );
        assert!(
            should_restart_popup_for_cwd(true, None, Some("/old"), "/repo"),
            "live process cwd remains the compatibility fallback"
        );
        assert!(
            should_restart_popup_for_cwd(true, None, Some("/repository"), "/repo"),
            "path components prevent sibling prefixes from being reused"
        );
        assert!(
            !should_restart_popup_for_cwd(true, None, None, "/repo"),
            "unknown popup cwd keeps reuse"
        );
        assert!(
            !should_restart_popup_for_cwd(false, Some("/old"), Some("/old/subdir"), "/repo"),
            "legacy visible popups keep their focus-derived cwd"
        );
    }

    #[test]
    // Defends: visible popup identity wins over a stale hidden candidate for the same spec.
    fn transient_selection_prefers_visible_popup_over_suppressed_popup() {
        let panes = [
            suppressed_transient_pane(10, "yzx_btm", Some("yzx popup_run btm")),
            transient_pane(11, "yzx_btm", Some("yzx popup_run btm"), false),
        ];

        assert_eq!(
            super::select_transient_pane_by_identity(
                &panes,
                super::TransientPaneIdentityView {
                    pane_title: "yzx_btm",
                    command_marker: Some("popup_run btm"),
                }
            ),
            Some(TransientPaneState {
                pane_id: 11,
                is_focused: false,
            })
        );
    }

    #[test]
    fn selects_displaced_configured_popup_panes_for_cleanup() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                top_popup {
                    command "yzx"
                    arg_1 "config"
                    arg_2 "ui"
                }
                bottom_popup {
                    command "lazygit"
                }
            "#,
        )]));
        let request = specs
            .request_from_message("toggle", Some("bottom_popup"))
            .expect("request");
        let panes = [
            transient_pane(11, "top_popup_popup", Some("yzx config ui"), false),
            transient_pane(22, "bottom_popup_popup", Some("lazygit"), true),
        ];

        assert_eq!(
            specs.select_other_configured_panes(&panes, request.spec.id.as_str(), Some(22)),
            vec![TransientPaneDisplacementCandidate {
                pane_id: 11,
                on_close: None,
                on_hide: None,
                toggle_close_behavior: TransientPopupToggleCloseBehavior::Close,
            }]
        );
    }

    #[test]
    // Regression: hidden keep-alive panes are live state, not displaced visible popup clutter.
    fn displaced_popup_cleanup_ignores_suppressed_popup_panes() {
        let specs = keep_alive_and_gitui_specs();
        let request = specs
            .request_from_message("toggle", Some("gitui"))
            .expect("request");
        let panes = [
            suppressed_transient_pane(10, "process_monitor_popup", Some("yzx popup_run btm")),
            transient_pane(11, "gitui_popup", Some("gitui"), true),
        ];

        assert_eq!(
            specs.select_other_configured_panes(&panes, request.spec.id.as_str(), Some(11)),
            Vec::<TransientPaneDisplacementCandidate<'_, i32>>::new()
        );
    }

    #[test]
    // Regression: displaced keep-alive popups must hide instead of losing process state.
    fn displaced_popup_cleanup_marks_visible_keep_alive_popup_for_hiding() {
        let specs = keep_alive_and_gitui_specs();
        let panes = [
            transient_pane(
                10,
                "process_monitor_popup",
                Some("yzx popup_run btm"),
                false,
            ),
            transient_pane(11, "gitui_popup", Some("gitui"), true),
        ];

        assert_eq!(
            specs.select_other_configured_panes(&panes, "gitui", Some(11)),
            vec![TransientPaneDisplacementCandidate {
                pane_id: 10,
                on_close: None,
                on_hide: None,
                toggle_close_behavior: TransientPopupToggleCloseBehavior::Hide,
            }]
        );
    }

    #[test]
    // Defends: displaced hide-mode popups carry their on_hide hook plan.
    fn displaced_keep_alive_popup_includes_on_hide_hook() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                process_monitor {
                    command "btm"
                    toggle_close_behavior "hide"
                    on_hide {
                        command "hook"
                        arg_1 "hidden"
                    }
                }
                gitui {
                    command "gitui"
                }
            "#,
        )]));
        let panes = [
            transient_pane(10, "process_monitor_popup", Some("btm"), false),
            transient_pane(11, "gitui_popup", Some("gitui"), true),
        ];
        let candidates = specs.select_other_configured_panes(&panes, "gitui", Some(11));

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].pane_id, 10);
        assert_eq!(
            candidates[0].toggle_close_behavior,
            TransientPopupToggleCloseBehavior::Hide
        );
        assert_eq!(
            candidates[0]
                .on_hide
                .and_then(|hook| hook.launch_plan("/repo"))
                .expect("hide hook plan")
                .command,
            vec!["hook", "hidden"]
        );
    }

    #[test]
    fn displaced_popup_cleanup_deduplicates_overlapping_identity_matches() {
        let specs = ConfiguredPopupSpecs::from_configuration(&config(&[(
            "popups",
            r#"
                first {
                    command "yzx"
                    command_marker "shared"
                    pane_title "shared_popup"
                }
                second {
                    command "shared"
                    command_marker "shared"
                    pane_title "also_shared_popup"
                }
                active {
                    command "lazygit"
                }
            "#,
        )]));
        let panes = [
            transient_pane(11, "shared_popup", Some("shared command"), false),
            transient_pane(22, "active_popup", Some("lazygit"), true),
        ];

        assert_eq!(
            specs.select_other_configured_panes(&panes, "active", Some(22)),
            vec![TransientPaneDisplacementCandidate {
                pane_id: 11,
                on_close: None,
                on_hide: None,
                toggle_close_behavior: TransientPopupToggleCloseBehavior::Close,
            }]
        );
    }
}
