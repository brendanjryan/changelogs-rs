use anyhow::Result;
use changelogs::Ecosystem;
use changelogs::changelog_entry;
use changelogs::config::Config;
use changelogs::workspace::Workspace;
use console::style;
use std::process::Command;

pub fn run(ecosystem: Option<Ecosystem>) -> Result<()> {
    println!("{} Running diagnostics...\n", style("→").blue().bold());

    let mut passed = 0;
    let mut failed = 0;

    let workspace = match Workspace::discover_with_ecosystem(ecosystem) {
        Ok(ws) => {
            println!(
                "  {} Workspace detected ({})",
                style("✓").green(),
                style(ws.root.display()).dim()
            );
            passed += 1;
            ws
        }
        Err(e) => {
            println!("  {} Workspace detection failed: {}", style("✗").red(), e);
            failed += 1;
            println!(
                "\n{} {passed} passed, {failed} failed",
                style("✗").red().bold()
            );
            return Ok(());
        }
    };

    if workspace.is_initialized() {
        println!("  {} Changelog directory initialized", style("✓").green());
        passed += 1;
    } else {
        println!(
            "  {} Changelog directory not initialized — run {}",
            style("✗").red(),
            style("changelogs init").cyan()
        );
        failed += 1;
        println!(
            "\n{} {passed} passed, {failed} failed",
            style("✗").red().bold()
        );
        return Ok(());
    }

    let changelog_dir = workspace.changelog_dir();
    let package_names: Vec<&str> = workspace.package_names();

    let config = match Config::load(&changelog_dir) {
        Ok(c) => {
            println!("  {} Config is valid", style("✓").green());
            passed += 1;
            c
        }
        Err(e) => {
            println!("  {} Config parse failed: {}", style("✗").red(), e);
            failed += 1;
            print_summary(passed, failed);
            return Ok(());
        }
    };

    for (i, group) in config.fixed.iter().enumerate() {
        let invalid: Vec<_> = group
            .members
            .iter()
            .filter(|m| !package_names.contains(&m.as_str()))
            .collect();
        if invalid.is_empty() {
            println!(
                "  {} Fixed group {} — all members valid",
                style("✓").green(),
                i + 1
            );
            passed += 1;
        } else {
            println!(
                "  {} Fixed group {} references unknown packages: {}",
                style("✗").red(),
                i + 1,
                invalid
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            failed += 1;
        }
    }

    for (i, group) in config.linked.iter().enumerate() {
        let invalid: Vec<_> = group
            .members
            .iter()
            .filter(|m| !package_names.contains(&m.as_str()))
            .collect();
        if invalid.is_empty() {
            println!(
                "  {} Linked group {} — all members valid",
                style("✓").green(),
                i + 1
            );
            passed += 1;
        } else {
            println!(
                "  {} Linked group {} references unknown packages: {}",
                style("✗").red(),
                i + 1,
                invalid
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            failed += 1;
        }
    }

    let invalid_ignores: Vec<_> = config
        .ignore
        .iter()
        .filter(|m| !package_names.contains(&m.as_str()))
        .collect();
    if invalid_ignores.is_empty() {
        println!("  {} Ignore list — all entries valid", style("✓").green());
        passed += 1;
    } else {
        println!(
            "  {} Ignore list references unknown packages: {}",
            style("✗").red(),
            invalid_ignores
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        failed += 1;
    }

    match changelog_entry::read_all(&changelog_dir) {
        Ok(changelogs) => {
            let mut invalid_refs: Vec<String> = Vec::new();
            for changelog in &changelogs {
                for release in &changelog.releases {
                    if !package_names.contains(&release.package.as_str()) {
                        invalid_refs.push(format!("'{}' in {}", release.package, changelog.id));
                    }
                }
            }
            if invalid_refs.is_empty() {
                println!(
                    "  {} Pending changelogs — all package references valid",
                    style("✓").green()
                );
                passed += 1;
            } else {
                println!(
                    "  {} Pending changelogs reference unknown packages:",
                    style("✗").red()
                );
                for r in &invalid_refs {
                    println!("      {}", style(r).dim());
                }
                failed += 1;
            }
        }
        Err(e) => {
            println!("  {} Failed to read changelogs: {}", style("✗").red(), e);
            failed += 1;
        }
    }

    let remote_ok = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);

    if remote_ok {
        println!("  {} Git remote detected", style("✓").green());
        passed += 1;
    } else {
        println!(
            "  {} Git remote not detected — changelog links will not include PR/commit references",
            style("✗").red()
        );
        failed += 1;
    }

    print_summary(passed, failed);
    Ok(())
}

fn print_summary(passed: usize, failed: usize) {
    println!();
    if failed > 0 {
        println!(
            "{} {passed} passed, {failed} failed",
            style("✗").red().bold()
        );
    } else {
        println!("{} All {passed} checks passed", style("✓").green().bold());
    }
}
