use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use tabled::{builder::Builder, settings::Style};

use codeowners::{get_diff, get_explain, get_owners, GitRef};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Owners { paths }) => cmd_owners(&paths),
        Some(Commands::Explain { path }) => cmd_explain(&path),
        Some(Commands::Diff { base_ref, head_ref }) => cmd_diff(&base_ref, &head_ref),
        None if cli.paths.is_empty() => {
            Cli::command().print_help()?;
            Ok(())
        }
        None => cmd_owners(&cli.paths),
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
}

#[derive(Subcommand)]
enum Commands {
    /// Show the owners for one or more paths.
    Owners {
        /// Paths to look up owners for.
        #[arg(required = true)]
        paths: Vec<String>,
    },

    /// Explain the CODEOWNERS assignment for a path.
    Explain {
        /// Path to explain ownership for.
        path: String,
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

fn cmd_owners(paths: &[String]) -> Result<()> {
    let owners = get_owners(paths)?;

    let rows: Vec<(String, String)> = owners
        .into_iter()
        .map(|(path, o)| (format!("`{path}`"), format_owners(&o)))
        .collect();

    println!("{}", build_markdown_table(&["path", "owners"], &rows));
    Ok(())
}

fn cmd_explain(path: &str) -> Result<()> {
    let (owners, rules) = get_explain(path)?;

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
