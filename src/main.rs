use std::io::{self, BufRead};

use anyhow::{bail, Result};
use clap::{CommandFactory, Parser, Subcommand};
use tabled::{builder::Builder, settings::Style};

use codeowners::{explain_owners, get_diff, get_owners, GitRef};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Owners {
            paths,
            stdin,
            no_check_path,
            filter,
        }) => cmd_owners(&resolve_paths(&paths, stdin)?, no_check_path, &filter),
        Some(Commands::Explain {
            path,
            no_check_path,
        }) => cmd_explain(&path, no_check_path),
        Some(Commands::Diff { base_ref, head_ref }) => cmd_diff(&base_ref, &head_ref),
        None if cli.paths.is_empty() && !cli.stdin => {
            Cli::command().print_help()?;
            Ok(())
        }
        None => cmd_owners(
            &resolve_paths(&cli.paths, cli.stdin)?,
            cli.no_check_path,
            &cli.filter,
        ),
    }
}

#[derive(Parser)]
#[command(about = "Tools for working with GitHub CODEOWNERS files")]
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Paths to look up owners for.
    paths: Vec<String>,

    /// Read paths from stdin (one per line).
    #[arg(long)]
    stdin: bool,

    /// Skip checking that paths exist.
    #[arg(long)]
    no_check_path: bool,

    /// Filter results by owner (comma-separated). Use "unowned" for unowned paths.
    #[arg(long, value_delimiter = ',')]
    filter: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show the owners for one or more paths.
    Owners {
        /// Paths to look up owners for.
        paths: Vec<String>,

        /// Read paths from stdin (one per line).
        #[arg(long)]
        stdin: bool,

        /// Skip checking that paths exist.
        #[arg(long)]
        no_check_path: bool,

        /// Filter results by owner (comma-separated). Use "unowned" for unowned paths.
        #[arg(long, value_delimiter = ',')]
        filter: Vec<String>,
    },

    /// Explain the CODEOWNERS assignment for a path.
    Explain {
        /// Path to explain ownership for.
        path: String,

        /// Skip checking that the path exists.
        #[arg(long)]
        no_check_path: bool,
    },

    /// Show how code ownership changes between two git refs.
    Diff {
        /// Base ref to compare from.
        #[arg(default_value = "HEAD")]
        base_ref: String,

        /// Head ref to compare to (default: the working tree).
        #[arg(default_value = "")]
        head_ref: String,
    },
}

/// Collect paths from args and/or stdin.
fn resolve_paths(paths: &[String], stdin: bool) -> Result<Vec<String>> {
    let mut result = paths.to_vec();
    if stdin {
        for line in io::stdin().lock().lines() {
            let line = line?;
            if !line.is_empty() {
                result.push(line);
            }
        }
    }
    if result.is_empty() {
        bail!("no paths provided");
    }
    Ok(result)
}

fn cmd_owners(paths: &[String], no_check_path: bool, filter: &[String]) -> Result<()> {
    let owners = get_owners(paths, !no_check_path, filter)?;

    let rows: Vec<(String, String)> = owners
        .into_iter()
        .map(|(path, o)| (format!("`{path}`"), format_owners(&o)))
        .collect();

    println!("{}", build_markdown_table(&["path", "owners"], &rows));
    Ok(())
}

fn cmd_explain(path: &str, no_check_path: bool) -> Result<()> {
    let (owners, rules) = explain_owners(path, !no_check_path)?;

    println!("Owners: {}\n", format_owners(&owners));

    if rules.is_empty() {
        println!("No matching rules.");
    } else {
        let mut builder = Builder::new();
        builder.push_record(["", "Line", "Pattern", "Owners", "Status"]);
        for rule in &rules {
            builder.push_record([
                if rule.active { "\u{2192}" } else { "" },
                &rule.line.to_string(),
                &rule.pattern,
                &format_owners(&rule.owners),
                if rule.active { "ACTIVE" } else { "superseded" },
            ]);
        }
        println!("{}", builder.build().with(Style::markdown()));
    }

    Ok(())
}

fn cmd_diff(base_ref: &str, head_ref: &str) -> Result<()> {
    let base = to_git_ref(base_ref);
    let head = to_git_ref(head_ref);

    let base_label = match &base {
        GitRef::WorkingTree => "working tree",
        GitRef::Ref(r) => r,
    };
    let head_label = match &head {
        GitRef::WorkingTree => "working tree",
        GitRef::Ref(r) => r,
    };

    let diff = get_diff(&base, &head)?;

    let mut printed_anything = false;

    if !diff.added.is_empty() {
        let rows: Vec<(String, String)> = diff
            .added
            .iter()
            .map(|(f, o)| (format!("`{f}`"), format_owners(o)))
            .collect();

        println!("## Added files ({} files)\n", rows.len());
        println!("{}", build_markdown_table(&["file", "owners"], &rows));
        printed_anything = true;
    }

    if !diff.removed.is_empty() {
        if printed_anything {
            println!();
        }
        let rows: Vec<(String, String)> = diff
            .removed
            .iter()
            .map(|(f, o)| (format!("`{f}`"), format_owners(o)))
            .collect();

        println!("## Removed files ({} files)\n", rows.len());
        println!("{}", build_markdown_table(&["file", "owners"], &rows));
        printed_anything = true;
    }

    if !diff.changed.is_empty() {
        if printed_anything {
            println!();
        }
        let rows: Vec<(String, String, String)> = diff
            .changed
            .iter()
            .map(|(f, base_o, head_o)| {
                (
                    format!("`{f}`"),
                    format_owners(base_o),
                    format_owners(head_o),
                )
            })
            .collect();

        println!("## Changed ownership ({} files)\n", rows.len());
        println!(
            "{}",
            build_markdown_table_3col(&["file", base_label, head_label], &rows)
        );
        printed_anything = true;
    }

    if !printed_anything {
        println!("No ownership changes.");
    }

    Ok(())
}

fn format_owners(owners: &[String]) -> String {
    if owners.is_empty() {
        "(unowned)".to_owned()
    } else {
        owners.join(", ")
    }
}

fn to_git_ref(s: &str) -> GitRef<'_> {
    if s.is_empty() {
        GitRef::WorkingTree
    } else {
        GitRef::Ref(s)
    }
}

fn build_markdown_table(headers: &[&str], rows: &[(String, String)]) -> String {
    let mut builder = Builder::new();
    builder.push_record(headers.iter().copied());
    for (a, b) in rows {
        builder.push_record([a.as_str(), b.as_str()]);
    }
    builder.build().with(Style::markdown()).to_string()
}

fn build_markdown_table_3col(headers: &[&str], rows: &[(String, String, String)]) -> String {
    let mut builder = Builder::new();
    builder.push_record(headers.iter().copied());
    for (a, b, c) in rows {
        builder.push_record([a.as_str(), b.as_str(), c.as_str()]);
    }
    builder.build().with(Style::markdown()).to_string()
}
