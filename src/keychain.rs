//! macOS Keychain inventory (service names only — never dumps secrets).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeychainClass {
    #[default]
    Unknown,
    Shared,
    Isolated,
}

impl KeychainClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Isolated => "isolated",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Default)]
pub struct KeychainReport {
    pub class: KeychainClass,
    pub services: Vec<String>,
    pub notes: Vec<String>,
}

pub fn inspect() -> KeychainReport {
    #[cfg(target_os = "macos")]
    {
        inspect_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        KeychainReport {
            class: KeychainClass::Unknown,
            services: vec![],
            notes: vec![
                "Keychain inspection is macOS-only; Linux/Windows use files under CLAUDE_CONFIG_DIR."
                    .into(),
            ],
        }
    }
}

#[cfg(target_os = "macos")]
fn inspect_macos() -> KeychainReport {
    use std::process::Command;

    let mut report = KeychainReport::default();

    let output = Command::new("security").args(["dump-keychain"]).output();
    let Ok(out) = output else {
        report
            .notes
            .push("failed to run `security dump-keychain`".into());
        return report;
    };

    let text = String::from_utf8_lossy(&out.stdout);
    let mut services = Vec::new();
    for line in text.lines() {
        if let Some(idx) = line.find("svce\"<blob>=\"") {
            let rest = &line[idx + "svce\"<blob>=\"".len()..];
            if let Some(end) = rest.find('"') {
                let svc = &rest[..end];
                if svc.contains("Claude Code") {
                    services.push(svc.to_string());
                }
            }
        }
    }
    services.sort();
    services.dedup();

    let has_plain = services.iter().any(|s| s == "Claude Code-credentials");
    let hashed: Vec<_> = services
        .iter()
        .filter(|s| {
            s.starts_with("Claude Code-credentials-") && s.as_str() != "Claude Code-credentials"
        })
        .cloned()
        .collect();

    report.services = services;
    report.class = if !hashed.is_empty() && !has_plain {
        KeychainClass::Isolated
    } else if !hashed.is_empty() && has_plain {
        report.notes.push(
            "both plain and hashed Claude Code-credentials services present — treat concurrent dual OAuth as unsafe".into(),
        );
        KeychainClass::Shared
    } else if has_plain {
        KeychainClass::Shared
    } else if !hashed.is_empty() {
        KeychainClass::Isolated
    } else {
        report.notes.push(
            "no Claude Code-credentials* services found yet (login may not have happened)".into(),
        );
        KeychainClass::Unknown
    };

    if report.class == KeychainClass::Shared {
        report.notes.push(
            "shared Keychain: do not run two OAuth profiles concurrently; use sequential sessions or setup-token/API for the second process".into(),
        );
    }
    if report.class == KeychainClass::Isolated {
        report.notes.push(
            "hashed Keychain services detected — concurrent OAuth may work; re-verify after Claude Code upgrades".into(),
        );
    }

    report
}

pub fn parallel_oauth_allowed(class: KeychainClass) -> bool {
    matches!(class, KeychainClass::Isolated)
}

pub fn redact_hint() -> &'static str {
    "silo never prints tokens or runs `security … -g`."
}
