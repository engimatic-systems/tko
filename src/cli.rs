// Generated from tko.org. Do not edit by hand.

use crate::storage::{TicketStore, migrate_legacy_properties};
use clap::{Args, CommandFactory, Parser, Subcommand};
use std::env;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "tko",
    version,
    about = "minimal org-mode ticket system",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print command help.
    Help,
    /// Create a ticket.
    Create(CreateArgs),
    /// Set status to in_progress.
    Start(IdArgs),
    /// Set status to blocked.
    Block(IdArgs),
    /// Set status to closed.
    Close(IdArgs),
    /// Set status to open.
    Reopen(IdArgs),
    /// Update ticket status.
    Status(StatusArgs),
    /// Add a dependency.
    Dep(RelationArgs),
    /// Remove a dependency.
    Undep(RelationArgs),
    /// Add a symmetric link.
    Link(RelationArgs),
    /// Remove a symmetric link.
    Unlink(RelationArgs),
    /// Add tag(s) to a ticket.
    Tag(TagsArgs),
    /// Remove tag(s) from a ticket.
    Untag(TagsArgs),
    /// List open or in-progress tickets with deps resolved.
    Ready(FilterArgs),
    /// List open or in-progress tickets with unresolved deps.
    Blocked(FilterArgs),
    /// List tickets.
    #[command(visible_alias = "ls")]
    List(ListArgs),
    /// Display ticket metadata and body outline.
    Show(ShowArgs),
    /// Append a timestamped note.
    #[command(name = "add-note")]
    AddNote(AddNoteArgs),
    /// Output tickets as JSON objects, optionally filtered.
    Query(QueryArgs),
    /// Validate semantic heading conventions.
    Lint(LintArgs),
    /// List note headings.
    Notes(IdArgs),
    /// Migrate legacy TK_* properties to TKO_* properties.
    #[command(name = "migrate-legacy-properties")]
    MigrateLegacyProperties(MigrationArgs),
}

#[derive(Debug, Args)]
struct CreateArgs {
    title: Option<String>,
    #[arg(short = 'd', long)]
    description: Option<String>,
    #[arg(long)]
    scope: Option<String>,
    #[arg(long)]
    design: Option<String>,
    #[arg(long)]
    acceptance: Option<String>,
    #[arg(short = 't', long = "type")]
    ticket_type: Option<String>,
    #[arg(short = 'p', long)]
    priority: Option<u8>,
    #[arg(short = 'a', long)]
    assignee: Option<String>,
    #[arg(long = "external-ref")]
    external_ref: Option<String>,
    #[arg(long)]
    parent: Option<String>,
    #[arg(long)]
    tags: Option<String>,
}

#[derive(Debug, Args)]
struct IdArgs {
    id: String,
}

#[derive(Debug, Args)]
struct StatusArgs {
    id: String,
    status: String,
}

#[derive(Debug, Args)]
struct RelationArgs {
    id: String,
    target_id: String,
}

#[derive(Debug, Args)]
struct TagsArgs {
    id: String,
    #[arg(required = true)]
    tags: Vec<String>,
}

#[derive(Debug, Args)]
struct FilterArgs {
    #[arg(short = 'a', long)]
    assignee: Option<String>,
    #[arg(short = 'T', long = "tag")]
    tag: Option<String>,
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(long)]
    status: Option<String>,
    #[command(flatten)]
    filters: FilterArgs,
}

#[derive(Debug, Args)]
struct ShowArgs {
    #[arg(short = 'f', long)]
    full: bool,
    id: String,
    #[arg(long)]
    note: Option<String>,
}

#[derive(Debug, Args)]
struct AddNoteArgs {
    id: String,
    text: Vec<String>,
}

#[derive(Debug, Args)]
struct QueryArgs {
    predicate: Vec<String>,
}

#[derive(Debug, Args)]
struct LintArgs {
    id_or_path: Option<String>,
}

#[derive(Debug, Args)]
struct MigrationArgs {
    #[arg(long)]
    apply: bool,
    id_or_path: Option<PathBuf>,
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Command::Help => "help",
            Command::Create(_) => "create",
            Command::Start(_) => "start",
            Command::Block(_) => "block",
            Command::Close(_) => "close",
            Command::Reopen(_) => "reopen",
            Command::Status(_) => "status",
            Command::Dep(_) => "dep",
            Command::Undep(_) => "undep",
            Command::Link(_) => "link",
            Command::Unlink(_) => "unlink",
            Command::Tag(_) => "tag",
            Command::Untag(_) => "untag",
            Command::Ready(_) => "ready",
            Command::Blocked(_) => "blocked",
            Command::List(_) => "list",
            Command::Show(_) => "show",
            Command::AddNote(_) => "add-note",
            Command::Query(_) => "query",
            Command::Lint(_) => "lint",
            Command::Notes(_) => "notes",
            Command::MigrateLegacyProperties(_) => "migrate-legacy-properties",
        }
    }
}

pub fn run_from<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match run(args) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            2
        }
    }
}

fn run<I, T>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    match cli.command {
        None | Some(Command::Help) => print_help().map_err(|error| error.to_string()),
        Some(Command::MigrateLegacyProperties(args)) => run_migration(args),
        Some(command) => Err(format!("not implemented: {}", command.name())),
    }
}

fn print_help() -> io::Result<()> {
    let mut command = Cli::command();
    command.print_long_help()?;
    println!();
    Ok(())
}

fn run_migration(args: MigrationArgs) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let tickets_dir_env = env::var_os("TICKETS_DIR").map(PathBuf::from);
    let store = TicketStore::discover_from(&cwd, tickets_dir_env.as_deref(), false)
        .map_err(|error| error.to_string())?;

    let paths = if let Some(target) = args.id_or_path {
        if target.exists() {
            vec![target]
        } else {
            let id = target.to_string_lossy();
            vec![store.resolve_id(&id).map_err(|error| error.to_string())?]
        }
    } else {
        store.ticket_paths().map_err(|error| error.to_string())?
    };

    let mut conflict_count = 0usize;
    for path in paths {
        let report =
            migrate_legacy_properties(&path, args.apply).map_err(|error| error.to_string())?;
        for action in report.actions {
            match action {
                crate::storage::MigrationAction::Rename {
                    legacy_key,
                    canonical_key,
                    value,
                } => println!(
                    "{}: rename {} -> {} ({})",
                    report.path.display(),
                    legacy_key,
                    canonical_key,
                    value
                ),
                crate::storage::MigrationAction::RemoveLegacy {
                    legacy_key,
                    canonical_key,
                    value,
                } => println!(
                    "{}: remove {} matching {} ({})",
                    report.path.display(),
                    legacy_key,
                    canonical_key,
                    value
                ),
            }
        }
        for conflict in report.conflicts {
            conflict_count += 1;
            eprintln!(
                "{}: conflict {}={} differs from {}={}",
                report.path.display(),
                conflict.legacy_key,
                conflict.legacy_value,
                conflict.canonical_key,
                conflict.canonical_value
            );
        }
    }

    if conflict_count == 0 {
        Ok(())
    } else {
        Err(format!(
            "legacy property migration found {conflict_count} conflict(s)"
        ))
    }
}
