// Generated from tko.org. Do not edit by hand.

use crate::read::{Filters, OutputMode};
use crate::storage::TicketStore;
use crate::write::CreateTicket;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
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
    /// Initialize ticket storage.
    Init,
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
    Ready(ReadArgs),
    /// List open or in-progress tickets with unresolved deps.
    Blocked(ReadArgs),
    /// List tickets.
    #[command(visible_alias = "ls")]
    List(ListArgs),
    /// Display ticket metadata/body, or one note with --note.
    Show(ShowArgs),
    /// Append a timestamped note.
    #[command(name = "add-note")]
    AddNote(AddNoteArgs),
    /// List tickets matching a predicate filter (summary; --output id|json).
    Query(QueryArgs),
    /// Validate semantic headings and lint rules L001-L003, including L003 note-title length.
    Lint(LintArgs),
    /// List note headings as timestamp plus title.
    Notes(IdArgs),
}

#[derive(Debug, Args)]
struct CreateArgs {
    title: String,
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
struct ReadArgs {
    #[command(flatten)]
    filters: FilterArgs,
    #[arg(long, value_enum, default_value_t = OutputArg::Summary)]
    output: OutputArg,
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(long)]
    status: Option<String>,
    #[command(flatten)]
    filters: FilterArgs,
    #[arg(long, value_enum, default_value_t = OutputArg::Summary)]
    output: OutputArg,
}

#[derive(Debug, Args)]
struct ShowArgs {
    #[arg(short = 'f', long)]
    full: bool,
    id: String,
    #[arg(long, help = "Print exactly one matching note subtree")]
    note: Option<String>,
}

#[derive(Debug, Args)]
struct AddNoteArgs {
    id: String,
    #[arg(long)]
    title: String,
    #[arg(long)]
    body: Option<String>,
}

#[derive(Debug, Args)]
struct QueryArgs {
    #[arg(long, value_enum, default_value_t = OutputArg::Summary, help = "Output format")]
    output: OutputArg,
    #[arg(
        value_name = "PREDICATE",
        help = "Filter expression; omit to match all tickets",
        long_help = "\
Filter expression; omit to match all tickets.

Grammar (keywords are case-sensitive):
  FIELD OP VALUE        OP: = != < <= > >=  (priority compares numerically)
  FIELD contain VALUE   membership test on a plural field (deps, links, tags)
  FIELD in [A, B, C]    scalar field equals any listed value
  has FIELD / no FIELD  field present / absent
  and  or  not  ( )     combine and group

Scalar fields: id status type assignee external-ref parent created title priority
Plural fields:  deps links tags

Examples:
  tko query status = open
  tko query priority <= 2 and status != closed
  tko query tags contain area/infra
  tko query status in [open, in_progress]
  tko query has parent and no assignee
  tko query (status = open or status = blocked) and priority <= 2"
    )]
    predicate: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputArg {
    Id,
    Summary,
    Json,
}

impl From<OutputArg> for OutputMode {
    fn from(output: OutputArg) -> Self {
        match output {
            OutputArg::Id => OutputMode::Id,
            OutputArg::Summary => OutputMode::Summary,
            OutputArg::Json => OutputMode::Json,
        }
    }
}

#[derive(Debug, Args)]
struct LintArgs {
    id_or_path: Option<String>,
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
        Some(Command::Init) => {
            let store = write_store(true)?;
            println!("Initialized {}", store.tickets_dir().display());
            Ok(())
        }
        Some(Command::Create(args)) => {
            let cwd = env::current_dir().map_err(|error| error.to_string())?;
            let store = write_store(false)?;
            let id = crate::write::create(&store, &cwd, create_ticket(args))
                .map_err(|error| error.to_string())?;
            println!("{id}");
            Ok(())
        }
        Some(Command::Status(args)) => print_write(crate::write::set_status(
            &write_store(false)?,
            &args.id,
            &args.status,
        )),
        Some(Command::Start(args)) => print_write(crate::write::set_status(
            &write_store(false)?,
            &args.id,
            "in_progress",
        )),
        Some(Command::Block(args)) => print_write(crate::write::set_status(
            &write_store(false)?,
            &args.id,
            "blocked",
        )),
        Some(Command::Close(args)) => print_write(crate::write::set_status(
            &write_store(false)?,
            &args.id,
            "closed",
        )),
        Some(Command::Reopen(args)) => print_write(crate::write::set_status(
            &write_store(false)?,
            &args.id,
            "open",
        )),
        Some(Command::Dep(args)) => print_write(crate::write::add_dependency(
            &write_store(false)?,
            &args.id,
            &args.target_id,
        )),
        Some(Command::Undep(args)) => print_write(crate::write::remove_dependency(
            &write_store(false)?,
            &args.id,
            &args.target_id,
        )),
        Some(Command::Link(args)) => print_write(crate::write::add_link(
            &write_store(false)?,
            &args.id,
            &args.target_id,
        )),
        Some(Command::Unlink(args)) => print_write(crate::write::remove_link(
            &write_store(false)?,
            &args.id,
            &args.target_id,
        )),
        Some(Command::Tag(args)) => print_write(crate::write::add_tags(
            &write_store(false)?,
            &args.id,
            &args.tags,
        )),
        Some(Command::Untag(args)) => print_write(crate::write::remove_tags(
            &write_store(false)?,
            &args.id,
            &args.tags,
        )),
        Some(Command::AddNote(args)) => print_write(crate::write::add_note(
            &write_store(false)?,
            &args.id,
            &args.title,
            args.body.as_deref(),
        )),
        Some(Command::Ready(args)) => {
            let store = read_store()?;
            let filters = filters(args.filters, None)?;
            print_read(crate::read::ready(&store, &filters, args.output.into()))
        }
        Some(Command::Blocked(args)) => {
            let store = read_store()?;
            let filters = filters(args.filters, None)?;
            print_read(crate::read::blocked(&store, &filters, args.output.into()))
        }
        Some(Command::List(args)) => {
            let store = read_store()?;
            let filters = filters(args.filters, args.status)?;
            print_read(crate::read::list(&store, &filters, args.output.into()))
        }
        Some(Command::Show(args)) => {
            let store = read_store()?;
            if let Some(note_match) = args.note {
                return print_note(crate::notes::show_note(&store, &args.id, &note_match));
            }
            print_read(crate::read::show(&store, &args.id, args.full))
        }
        Some(Command::Query(args)) => {
            let store = read_store()?;
            let predicate = args.predicate.join(" ");
            print_read(crate::read::query(
                &store,
                Some(&predicate),
                args.output.into(),
            ))
        }
        Some(Command::Lint(args)) => run_lint(args),
        Some(Command::Notes(args)) => {
            print_note(crate::notes::list_notes(&read_store()?, &args.id))
        }
    }
}

fn print_help() -> io::Result<()> {
    let mut command = Cli::command();
    command.print_long_help()?;
    println!();
    Ok(())
}

fn read_store() -> Result<TicketStore, String> {
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let tickets_dir_env = env::var_os("TICKETS_DIR").map(PathBuf::from);
    TicketStore::discover_from(&cwd, tickets_dir_env.as_deref(), false)
        .map_err(|error| error.to_string())
}

fn write_store(create_if_missing: bool) -> Result<TicketStore, String> {
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let tickets_dir_env = env::var_os("TICKETS_DIR").map(PathBuf::from);
    TicketStore::discover_from(&cwd, tickets_dir_env.as_deref(), create_if_missing)
        .map_err(|error| error.to_string())
}

fn create_ticket(args: CreateArgs) -> CreateTicket {
    CreateTicket {
        title: args.title,
        description: args.description,
        scope: args.scope,
        design: args.design,
        acceptance: args.acceptance,
        ticket_type: args.ticket_type.unwrap_or_else(|| "task".to_string()),
        priority: args.priority.unwrap_or(2),
        assignee: args.assignee,
        external_ref: args.external_ref,
        parent: args.parent,
        tags: split_tags(args.tags),
    }
}

fn split_tags(tags: Option<String>) -> Vec<String> {
    tags.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn filters(args: FilterArgs, status: Option<String>) -> Result<Filters, String> {
    Ok(Filters {
        status,
        assignee: args.assignee,
        tag: args.tag,
    })
}

fn print_read(result: crate::read::Result<String>) -> Result<(), String> {
    let output = result.map_err(|error| error.to_string())?;
    print!("{output}");
    Ok(())
}

fn print_write(result: crate::write::Result<String>) -> Result<(), String> {
    let output = result.map_err(|error| error.to_string())?;
    print!("{output}");
    Ok(())
}

fn print_note(result: crate::notes::Result<String>) -> Result<(), String> {
    let output = result.map_err(|error| error.to_string())?;
    print!("{output}");
    Ok(())
}

fn run_lint(args: LintArgs) -> Result<(), String> {
    let store = read_store()?;
    let findings = if let Some(id_or_path) = args.id_or_path {
        let path = crate::lint::resolve_id_or_path(&store, &id_or_path)
            .map_err(|error| error.to_string())?;
        crate::lint::lint_path(&path)
    } else {
        crate::lint::lint_store(&store)
    }
    .map_err(|error| error.to_string())?;

    for finding in &findings {
        println!("{finding}");
    }

    if crate::lint::has_failures(&findings) {
        Err("lint failed".to_string())
    } else {
        Ok(())
    }
}
