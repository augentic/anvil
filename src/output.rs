use crate::registry::{Registry, Service};
use crate::status::PipelineStatus;

/// Print the dry-run banner for a command.
pub fn dry_run_banner(command: &str, change: &str) {
    println!("=== DRY RUN: {command} '{change}' ===\n");
}

/// Print the dry-run footer.
pub fn dry_run_footer() {
    println!("\nno changes made (dry run)");
}

/// Print all services in a tabular format.
pub fn print_registry(reg: &Registry) {
    println!("{:<24} {:<12} {:<24} REPO", "ID", "DOMAIN", "CRATE");
    println!("{}", "-".repeat(80));
    for s in &reg.services {
        println!("{:<24} {:<12} {:<24} {}", s.id, s.domain, s.crate_name, s.repo);
    }
}

/// Print services matching a domain filter.
pub fn print_services_by_domain(services: &[&Service], domain: &str) {
    if services.is_empty() {
        println!("no services in domain '{domain}'");
        return;
    }
    println!("services in domain '{domain}':");
    for s in services {
        println!(
            "  {:<24} crate={:<20} caps=[{}]",
            s.id,
            s.crate_name,
            s.capabilities.join(", ")
        );
    }
}

/// Print services matching a capability filter.
pub fn print_services_by_capability(services: &[&Service], cap: &str) {
    if services.is_empty() {
        println!("no services with capability '{cap}'");
        return;
    }
    println!("services with capability '{cap}':");
    for s in services {
        println!("  {:<24} domain={:<12} crate={}", s.id, s.domain, s.crate_name);
    }
}

/// Print the pipeline status summary table.
pub fn print_status_summary(status: &PipelineStatus) {
    use crate::status::TargetState;

    println!("change: {}", status.change);
    println!("updated: {}", status.updated);
    println!();
    println!("{:<24} {:<14} PR", "TARGET", "STATE");
    println!("{}", "-".repeat(72));
    for t in &status.targets {
        println!(
            "{:<24} {:<14} {}",
            t.id,
            t.state,
            t.pr.as_deref().unwrap_or("-")
        );
    }
    let done = status
        .targets
        .iter()
        .filter(|t| t.state.is_at_least(TargetState::Implemented))
        .count();
    println!();
    println!(
        "progress: {}/{} targets implemented or later",
        done,
        status.targets.len()
    );
}
