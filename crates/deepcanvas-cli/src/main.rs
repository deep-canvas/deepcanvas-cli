use clap::{Parser, Subcommand};
use clap_complete::Shell;
use deepcanvas_core::Config;

mod commands;
mod ui;

#[derive(Parser)]
#[command(name = "deep", version, about = "DeepCanvas CLI")]
pub struct Cli {
    /// Machine-readable JSON output, no interactivity
    #[arg(long, global = true)]
    pub headless: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate this device with DeepCanvas
    Login,
    /// Remove stored credentials from this device
    Logout,
    /// Link the current directory to a DeepCanvas project
    Init {
        /// Project as <org-slug>/<project-slug>. Omit for interactive picker.
        slug_pair: Option<String>,
    },
    /// List assigned tasks
    Tasks {
        #[arg(long, short)]
        project: Option<String>,
    },
    /// Pull task context into .deep/<task-code>/
    Pull {
        #[arg(required = true)]
        task_codes: Vec<String>,
        #[arg(long, short)]
        project: Option<String>,
    },
    /// Mark a task as done
    Done {
        /// Task code, e.g. DC-142. Omit to complete the active task.
        task_code: Option<String>,
    },
    /// Generate shell completion script
    Completion { shell: Shell },
    /// Self-update the deep binary from GitHub releases
    Update,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = Config::load();

    if cli.headless {
        colored::control::set_override(false);
    }

    let result = match cli.command {
        Commands::Login => commands::login::run(config, cli.headless).await,
        Commands::Logout => commands::logout::run(cli.headless),
        Commands::Init { slug_pair } => commands::init::run(config, slug_pair, cli.headless).await,
        Commands::Tasks { project } => commands::tasks::run(config, project, cli.headless).await,
        Commands::Pull {
            task_codes,
            project,
        } => commands::pull::run(config, task_codes, project, cli.headless).await,
        Commands::Done { task_code } => commands::done::run(config, task_code, cli.headless).await,
        Commands::Completion { shell } => {
            commands::completion::run(shell);
            Ok(())
        }
        Commands::Update => commands::update::run(cli.headless).await,
    };

    if let Err(e) = result {
        if cli.headless {
            ui::print_error_json(&e);
        } else {
            ui::print_error(&e);
        }
        std::process::exit(1);
    }
}
