use std::path::PathBuf;

use clap::Parser;
use colored::Colorize;
use integration_tests_sv2::start_tracing;
use interoperability_tests_sv2::{
    app_type::AppType,
    config::{Config, Target, TargetAddress},
    runner::run_suite,
    scenarios::{ScenarioError, ScenarioReport},
};

#[derive(Parser)]
#[command(
    name = "sv2-compliance",
    about = "Sv2 interoperability compliance tester"
)]
struct Cli {
    /// Path to the TOML configuration file listing sites to test
    #[arg(short = 'c', long, default_value = "targets.toml")]
    config: PathBuf,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Default)]
struct RunSummary {
    passed: u32,
    failed: u32,
    not_exercised: u32,
}
impl RunSummary {
    fn record(&mut self, report: &ScenarioReport) {
        match &report.result {
            Ok(_) => self.passed += 1,
            Err(ScenarioError::NotExercised(_)) => self.not_exercised += 1,
            Err(ScenarioError::Failed(_)) => self.failed += 1,
        }
    }

    fn succeeded(&self, unresolved: usize) -> bool {
        self.failed == 0 && unresolved == 0
    }

    fn summary_line(&self, resolved: usize, unresolved: usize) -> String {
        if resolved == 0 && unresolved > 0 {
            return format!(
                "{} no targets resolved ({} unresolved)",
                "✗".red().bold(),
                unresolved
            );
        }

        let total = self.passed + self.failed + self.not_exercised;
        let failed_note = if self.failed > 0 {
            format!(", {} failed", self.failed)
        } else {
            String::new()
        };
        let not_exercised_note = if self.not_exercised > 0 {
            format!(", {} not exercised", self.not_exercised)
        } else {
            String::new()
        };
        let unresolved_note = if unresolved > 0 {
            format!("; {unresolved} target(s) unresolved")
        } else {
            String::new()
        };

        if self.succeeded(unresolved) {
            format!(
                "{} {}/{} scenarios passed{} across {} resolved target(s)",
                "✓".green().bold(),
                self.passed,
                total,
                not_exercised_note,
                resolved
            )
        } else {
            format!(
                "{} {}/{} scenarios passed{}{}{} across {} resolved target(s)",
                "✗".red().bold(),
                self.passed,
                total,
                failed_note,
                not_exercised_note,
                unresolved_note,
                resolved
            )
        }
    }

    fn print(&self, resolved: usize, unresolved: usize) {
        println!("{}", self.summary_line(resolved, unresolved));
    }
}

#[tokio::main]
async fn main() {
    let mut summary = RunSummary::default();

    let cli = Cli::parse();
    if cli.verbose {
        start_tracing();
    }

    let targets = match Config::from_path(&cli.config) {
        Ok(config) => config.resolve_targets().await,
        Err(e) => {
            eprintln!("{} {e}", "error:".red().bold());
            std::process::exit(1);
        }
    };

    if targets.is_empty() {
        eprintln!("{} no endpoints configured to test", "error:".red().bold());
        std::process::exit(1);
    }

    for target in &targets.resolved {
        print_target_header(
            &target.site_name,
            target.endpoint.app_type,
            format!("{}{}", target.address, resolved_note(target)),
        );
        let reports = run_suite(&target.endpoint).await;
        for report in &reports {
            summary.record(report);
            print_report(report);
        }
        println!();
    }

    for target in &targets.unresolved {
        print_target_header(
            &target.site_name,
            target.app_type,
            target.address.to_string(),
        );
        println!(
            "  {} target unresolved: {}",
            "✗".red(),
            target.error.to_string().red()
        );
        println!();
    }

    summary.print(targets.resolved.len(), targets.unresolved.len());

    if !summary.succeeded(targets.unresolved.len()) {
        std::process::exit(1);
    }
}

fn print_target_header(site_name: &str, app_type: AppType, address: String) {
    println!(
        "── {} ──",
        format!("{site_name}: {app_type:?} at {address}")
            .blue()
            .bold()
    );
}

/// Shows the resolved IP when the configured address was a hostname.
fn resolved_note(target: &Target) -> String {
    match &target.address {
        TargetAddress::Socket(_) => String::new(),
        TargetAddress::Hostname { .. } => {
            format!(" (resolved to {})", target.endpoint.addr)
        }
    }
}

fn print_report(report: &ScenarioReport) {
    match &report.result {
        Ok(detail) => {
            let detail = detail
                .as_deref()
                .map(|d| format!(" ({})", d.dimmed()))
                .unwrap_or_default();
            println!("  {} {}{detail}", "✓".green(), report.id.blue());
        }
        Err(ScenarioError::NotExercised(reason)) => println!(
            "  {} {}: {}",
            "○".dimmed(),
            report.id.blue(),
            format!("not exercised: {reason}").dimmed()
        ),
        Err(e) => println!(
            "  {} {}: {}",
            "✗".red(),
            report.id.blue(),
            e.to_string().red()
        ),
    }
}
