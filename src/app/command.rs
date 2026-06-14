/// Resolve a command input string to a view identifier.
pub(crate) fn resolve_view(input: &str) -> Option<String> {
    match input.trim().to_lowercase().as_str() {
        "" => None,
        "vm" | "vms" | "qemu" => Some("qemu".to_string()),
        "node" | "nodes" => Some("node".to_string()),
        "ct" | "container" | "containers" | "lxc" => Some("lxc".to_string()),
        "storage" | "storages" => Some("storage".to_string()),
        _ => None,
    }
}

/// Tab-completion suffix for view commands.
pub fn view_completion(input: &str) -> Option<&'static str> {
    const COMPLETIONS: &[&str] = &[
        "vm",
        "vms",
        "qemu",
        "node",
        "nodes",
        "ct",
        "container",
        "containers",
        "lxc",
        "storage",
        "storages",
    ];

    let lower = input.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    COMPLETIONS
        .iter()
        .find(|v| v.starts_with(&lower))
        .map(|v| &v[lower.len()..])
}

/// Extract the numeric VM/container ID from a Proxmox resource identifier
/// (e.g. `"qemu/100"` → `Some(100)`).
pub(crate) fn extract_vmid(id: &str) -> Option<u32> {
    id.split('/').nth(1)?.parse().ok()
}
