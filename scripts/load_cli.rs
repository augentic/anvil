// Shared `cli` resolution for scripts/specify.rs and scripts/use-local-dev.rs (include!).

fn read_cli_spec() -> Option<toml::Table> {
    ["Specify.local.toml", "Specify.toml"]
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .filter_map(|s| s.parse::<toml::Table>().ok())
        .find_map(|t| t.get("cli")?.as_table().cloned())
}
