use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use cosmic_mime_apps::{apps_for_mime, associations, List};
use mime::Mime;

/// Registry that resolves MIME types to desktop applications using layered mimeapps.list files.
///
/// Precedence: user (~/.config/mimeapps.list) > admin (/etc/xdg/mimeapps.list) > system (/usr/share/applications/mimeapps.list).
#[derive(Clone)]
pub struct MimeRegistry {
    apps: Arc<BTreeMap<Arc<str>, Arc<cosmic_mime_apps::App>>>,
    lists: Arc<List>,
}

impl MimeRegistry {
    /// Load registry from the standard override paths.
    pub fn load_default() -> Self {
        let paths = default_mimeapps_paths();

        let mut lists = List::default();
        lists.load_from_paths(&paths);

        let apps = associations::by_app();

        Self {
            apps: Arc::new(apps),
            lists: Arc::new(lists),
        }
    }

    /// Resolve the default application desktop ID for a MIME type.
    pub fn resolve(&self, mime: &str) -> Option<String> {
        let mime: Mime = mime.parse().ok()?;

        if let Some(defaults) = self.lists.default_app_for(&mime) {
            if let Some(first) = defaults.first() {
                return Some(first.to_string());
            }
        }

        // Fallback to the first associated app we know about.
        let first = apps_for_mime(&mime, &self.apps).next().map(|(id, _)| id.to_string());
        first
    }

    /// List all handlers (desktop IDs) that can open the given MIME type.
    pub fn list_handlers(&self, mime: &str) -> Vec<String> {
        let mime: Mime = match mime.parse() {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        let mut seen = std::collections::BTreeSet::new();
        let mut out = Vec::new();

        if let Some(defaults) = self.lists.default_app_for(&mime) {
            for app in defaults {
                if seen.insert(app.as_ref()) {
                    out.push(app.to_string());
                }
            }
        }

        if let Some(added) = self.lists.added_associations.get(&mime) {
            for app in added {
                if seen.insert(app.as_ref()) {
                    out.push(app.to_string());
                }
            }
        }

        for (id, _) in apps_for_mime(&mime, &self.apps) {
            if seen.insert(id.as_ref()) {
                out.push(id.to_string());
            }
        }

        out
    }

    /// Launch the given desktop entry with the provided file path.
    pub fn launch(&self, desktop_id: &str, file: &Path) -> anyhow::Result<()> {
        let app = self
            .apps
            .iter()
            .find(|(id, _)| id.as_ref() == desktop_id)
            .map(|(_, app)| app.clone())
            .ok_or_else(|| anyhow::anyhow!("Desktop entry not found: {}", desktop_id))?;

        let exec = read_exec_line(&app.path)
            .ok_or_else(|| anyhow::anyhow!("Missing Exec line for {}", desktop_id))?;

        let file_arg = file.to_string_lossy().to_string();
        let mut parts: Vec<String> = exec.split_whitespace().map(|s| s.to_string()).collect();
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Invalid Exec line for {}", desktop_id));
        }

        // Replace field codes; append path if none present.
        let mut had_field_code = false;
        for part in parts.iter_mut() {
            if part.contains("%f") || part.contains("%u") {
                *part = part.replace("%f", &file_arg).replace("%u", &file_arg);
                had_field_code = true;
            } else if part.contains("%F") || part.contains("%U") {
                *part = part.replace("%F", &file_arg).replace("%U", &file_arg);
                had_field_code = true;
            } else if part.contains("%c") {
                *part = part.replace("%c", app.name.as_ref());
            } else if part.contains("%i") {
                // Drop %i or replace with icon flag if desired; here we drop.
                *part = part.replace("%i", "");
            } else if part.contains("%k") {
                *part = part.replace("%k", app.path.to_string_lossy().as_ref());
            }
        }
        if !had_field_code {
            parts.push(file_arg);
        }

        parts.retain(|p| !p.is_empty());

        let program = parts.remove(0);
        let mut cmd = std::process::Command::new(program);
        if !parts.is_empty() {
            cmd.args(parts);
        }

        cmd.spawn()
            .map_err(|e| anyhow::anyhow!("Failed to launch {}: {}", desktop_id, e))?;
        Ok(())
    }
}

fn default_mimeapps_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    if let Some(home) = dirs::config_dir() {
        let user = home.join("mimeapps.list");
        if user.exists() {
            paths.push(user);
        }
    }

    let admin = std::path::PathBuf::from("/etc/xdg/mimeapps.list");
    if admin.exists() {
        paths.push(admin);
    }

    let system = std::path::PathBuf::from("/usr/share/applications/mimeapps.list");
    if system.exists() {
        paths.push(system);
    }

    paths
}

fn read_exec_line(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Exec=") {
            return Some(trimmed.trim_start_matches("Exec=").trim().to_string());
        }
    }
    None
}
