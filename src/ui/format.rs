/// Human-readable label for a resource view type.
pub fn view_label(view: &str) -> &str {
    match view {
        "qemu" => "VMs",
        "lxc" => "containers",
        other => other,
    }
}

/// Format memory as `used / total GB`.
pub fn format_memory(used: Option<u64>, total: Option<u64>) -> String {
    match (used, total) {
        (Some(u), Some(t)) => format!("{:.1} / {:.1} GB", u as f64 / 1e9, t as f64 / 1e9),
        _ => "-".to_string(),
    }
}

/// Format disk as `used / total GB` (integer values).
pub fn format_disk(used: Option<u64>, total: Option<u64>) -> String {
    match (used, total) {
        (Some(u), Some(t)) => {
            format!(
                "{} / {} GB",
                u / (1024 * 1024 * 1024),
                t / (1024 * 1024 * 1024)
            )
        }
        _ => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_label_maps_known_types() {
        assert_eq!(view_label("node"), "node");
        assert_eq!(view_label("qemu"), "VMs");
        assert_eq!(view_label("lxc"), "containers");
        assert_eq!(view_label("storage"), "storage");
    }

    #[test]
    fn view_label_passes_through_unknown() {
        assert_eq!(view_label("sdn"), "sdn");
    }
}
