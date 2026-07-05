use crate::decisions::is_scratch;
use crate::herdr::PaneInfo;
use crate::state::ScratchState;

pub const WORKSPACE_MARKER_SUFFIX: &str = " [scratch-on]";

pub fn marked_workspace_label(label: &str) -> String {
    if label.ends_with(WORKSPACE_MARKER_SUFFIX) {
        label.to_string()
    } else {
        format!("{label}{WORKSPACE_MARKER_SUFFIX}")
    }
}

pub fn original_workspace_label(label: &str) -> String {
    label
        .strip_suffix(WORKSPACE_MARKER_SUFFIX)
        .unwrap_or(label)
        .to_string()
}

pub fn restore_workspace_label(state: &ScratchState, current_label: &str) -> Option<String> {
    match (
        state.original_workspace_label.as_deref(),
        state.marked_workspace_label.as_deref(),
    ) {
        (Some(original), Some(marked)) if current_label == marked => Some(original.to_string()),
        (None, None) if current_label.ends_with(WORKSPACE_MARKER_SUFFIX) => {
            Some(original_workspace_label(current_label))
        }
        _ => None,
    }
}

pub fn legacy_marker_cleanup_target(
    state: Option<&ScratchState>,
    panes: &[PaneInfo],
    current: &PaneInfo,
) -> Option<String> {
    if let Some(state) = state {
        if panes.iter().any(|pane| pane.pane_id == state.host_pane_id) {
            return Some(state.host_pane_id.clone());
        }
    }

    let workspace = state
        .and_then(|state| state.workspace_id.as_deref())
        .or(current.workspace_id.as_deref());

    panes
        .iter()
        .find(|pane| {
            !is_scratch(pane)
                && match workspace {
                    Some(workspace) => pane.workspace_id.as_deref() == Some(workspace),
                    None => true,
                }
        })
        .map(|pane| pane.pane_id.clone())
        .or_else(|| (!is_scratch(current)).then(|| current.pane_id.clone()))
}
