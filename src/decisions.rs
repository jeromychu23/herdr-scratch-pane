use crate::herdr::PaneInfo;
use crate::scope::{scratch_label, Scope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleInputs {
    pub scope: Scope,
    pub current: PaneInfo,
    pub panes: Vec<PaneInfo>,
    pub server_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToggleDecision {
    Open { scope: Scope },
    Reveal { pane_id: String },
    Close { pane_id: String },
    CloseThenOpen { close_pane_id: String, scope: Scope },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinimizeDecision {
    Close { pane_id: String },
    NotifyNoVisiblePane,
}

pub fn decide_toggle(input: ToggleInputs) -> ToggleDecision {
    let target_label = scratch_label(input.scope);
    let current_workspace = input.current.workspace_id.as_deref();

    let target = input.panes.iter().find(|pane| {
        pane.label.as_deref() == Some(target_label)
            && match input.scope {
                Scope::Workspace => pane.workspace_id.as_deref() == current_workspace,
                Scope::Session => true,
            }
    });

    if let Some(pane) = target {
        if pane.focused || pane.pane_id == input.current.pane_id {
            return ToggleDecision::Close {
                pane_id: pane.pane_id.clone(),
            };
        }
    }

    if let Some(other_visible) = input.panes.iter().find(|pane| {
        pane.focused && is_scratch(pane) && pane.label.as_deref() != Some(target_label)
    }) {
        return ToggleDecision::CloseThenOpen {
            close_pane_id: other_visible.pane_id.clone(),
            scope: input.scope,
        };
    }

    if let Some(pane) = target {
        if input.scope == Scope::Session && pane.workspace_id.as_deref() != current_workspace {
            return ToggleDecision::CloseThenOpen {
                close_pane_id: pane.pane_id.clone(),
                scope: input.scope,
            };
        }
        return ToggleDecision::Reveal {
            pane_id: pane.pane_id.clone(),
        };
    }

    ToggleDecision::Open { scope: input.scope }
}

pub fn minimize_decision(current: &PaneInfo, panes: &[PaneInfo]) -> MinimizeDecision {
    if is_scratch(current) || current.label.as_deref() == Some(scratch_label(Scope::Workspace)) {
        return MinimizeDecision::Close {
            pane_id: current.pane_id.clone(),
        };
    }

    if let Some(focused) = panes.iter().find(|pane| pane.focused && is_scratch(pane)) {
        return MinimizeDecision::Close {
            pane_id: focused.pane_id.clone(),
        };
    }

    MinimizeDecision::NotifyNoVisiblePane
}

pub fn open_target_for_current(current: &PaneInfo) -> Option<String> {
    (!is_scratch(current)).then(|| current.pane_id.clone())
}

pub fn is_scratch(pane: &PaneInfo) -> bool {
    matches!(
        pane.label.as_deref(),
        Some("⌂ scratch workspace") | Some("⌂ scratch session")
    )
}
