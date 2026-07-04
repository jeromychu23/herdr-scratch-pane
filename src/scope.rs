#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Workspace,
    Session,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Session => "session",
        }
    }
}

pub fn scratch_label(scope: Scope) -> &'static str {
    match scope {
        Scope::Workspace => "⌂ scratch workspace",
        Scope::Session => "⌂ scratch session",
    }
}

pub fn session_name(scope: Scope, workspace_id: Option<&str>, server_id: Option<&str>) -> String {
    match scope {
        Scope::Workspace => format!("workspace-{}", sanitize(workspace_id.unwrap_or("default"))),
        Scope::Session => format!("session-{}", sanitize(server_id.unwrap_or("default"))),
    }
}

fn sanitize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "default".into()
    } else {
        trimmed.into()
    }
}
