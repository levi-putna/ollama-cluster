use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ocluster_client::ManagementClient;
use ocluster_config::{
    default_config_path, init_config, load_config, ConfigOverrides,
};
use ocluster_core::ModelMode;
use ocluster_protocol::AddNodeRequest;

use crate::output::{OutputFormat, OutputWriter};

/// Ollama Cluster — intelligent routing for multiple Ollama instances.
#[derive(Debug, Parser)]
#[command(name = "ocluster", version, about)]
pub struct Cli {
    /// Management API endpoint.
    #[arg(long, env = "OCLUSTER_MANAGEMENT_URL", default_value = "http://127.0.0.1:11600")]
    pub endpoint: String,

    /// Path to configuration file.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Output format for read commands.
    #[arg(long, global = true, value_enum, default_value = "table")]
    pub output: OutputFormat,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// CLI subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    /// Launch the interactive terminal dashboard.
    Dashboard,
    /// Launch the web admin panel.
    Admin {
        /// Admin panel listen address.
        #[arg(long, default_value = "127.0.0.1:11602")]
        listen: String,
    },
    /// Initialise cluster configuration.
    Init {
        /// Output config path.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Inference listen address.
        #[arg(long)]
        inference_listen: Option<String>,
        /// Management listen address.
        #[arg(long)]
        management_listen: Option<String>,
        /// Database path.
        #[arg(long)]
        database_path: Option<String>,
        /// Register a node during init (name:url).
        #[arg(long = "node")]
        nodes: Vec<String>,
        /// Non-interactive mode.
        #[arg(long)]
        non_interactive: bool,
    },
    /// Run the cluster controller.
    Serve,
    /// Show cluster status.
    Status,
    /// List nodes.
    Nodes,
    /// Show health summary.
    Health,
    /// List cluster events.
    Events,
    /// Node management commands.
    Node {
        #[command(subcommand)]
        command: NodeCommands,
    },
    /// Model commands.
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },
    /// List models.
    Models,
    /// Explain routing for a model.
    Explain {
        /// Model name.
        model: String,
    },
    /// Request monitoring commands.
    Requests {
        #[command(subcommand)]
        command: Option<RequestCommands>,
    },
    /// Configuration commands.
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Show controller logs (placeholder — logs go to stderr when serving).
    Logs {
        #[arg(long)]
        follow: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum NodeCommands {
    Add {
        name: String,
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "discover")]
        model_mode: String,
    },
    Remove {
        name: String,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        yes: bool,
    },
    Enable { name: String },
    Disable { name: String },
    Drain { name: String },
    Inspect { name: String },
    Probe { name: String },
    Models {
        #[command(subcommand)]
        command: NodeModelCommands,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum NodeModelCommands {
    Sync {
        name: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ModelCommands {
    Inspect { name: String },
}

#[derive(Debug, Clone, Subcommand)]
pub enum RequestCommands {
    Watch,
    Cancel { id: String },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommands {
    Show,
    Validate,
    Reload,
}

/// Execute a CLI command.
pub async fn execute(cmd: Commands, cli: &Cli) -> Result<()> {
    match cmd {
        Commands::Dashboard => cmd_dashboard(cli).await,
        Commands::Admin { listen } => cmd_admin(cli, listen).await,
        Commands::Init {
            path,
            inference_listen,
            management_listen,
            database_path,
            nodes,
            non_interactive: _,
        } => cmd_init(path, inference_listen, management_listen, database_path, nodes).await,
        Commands::Serve => cmd_serve(cli).await,
        Commands::Status => cmd_with_client(cli, |c, out| async move {
            let status = c.cluster_status().await?;
            out.write_cluster_status(&status);
            Ok(())
        })
        .await,
        Commands::Nodes => cmd_with_client(cli, |c, out| async move {
            let nodes = c.list_nodes().await?;
            out.write_nodes(&nodes);
            Ok(())
        })
        .await,
        Commands::Health => cmd_with_client(cli, |c, out| async move {
            let nodes = c.list_nodes().await?;
            out.write_health(&nodes);
            Ok(())
        })
        .await,
        Commands::Events => cmd_with_client(cli, |c, out| async move {
            let events = c.list_events().await?;
            out.write_events(&events);
            Ok(())
        })
        .await,
        Commands::Node { command } => execute_node(command, cli).await,
        Commands::Model { command } => execute_model(command, cli).await,
        Commands::Models => cmd_with_client(cli, |c, out| async move {
            let models = c.list_models().await?;
            out.write_models(&models);
            Ok(())
        })
        .await,
        Commands::Explain { model } => cmd_with_client(cli, |c, out| async move {
            let explain = c.explain_model(&model).await?;
            out.write_explain(&explain);
            Ok(())
        })
        .await,
        Commands::Requests { command } => execute_requests(command, cli).await,
        Commands::Config { command } => execute_config(command, cli).await,
        Commands::Logs { follow } => {
            if follow {
                eprintln!("log follow is not available in 0.1.0; run `ocluster serve` to view logs on stderr");
            } else {
                eprintln!("logs are written to stderr when running `ocluster serve`");
            }
            Ok(())
        }
    }
}

/// Launch the interactive Ratatui dashboard.
pub async fn cmd_dashboard(cli: &Cli) -> Result<()> {
    tokio::task::spawn_blocking({
        let endpoint = cli.endpoint.clone();
        move || ocluster_tui::run_dashboard(&endpoint)
    })
    .await
    .context("dashboard task failed")??;
    Ok(())
}

/// Launch the web admin panel.
pub async fn cmd_admin(cli: &Cli, listen: String) -> Result<()> {
    println!("Admin panel: http://{listen}");
    println!("Management API: {}", cli.endpoint);
    ocluster_admin::run_admin_server(&listen, &cli.endpoint)
        .await
        .context("admin panel exited with error")
}

async fn cmd_init(
    path: Option<PathBuf>,
    inference_listen: Option<String>,
    management_listen: Option<String>,
    database_path: Option<String>,
    nodes: Vec<String>,
) -> Result<()> {
    let config_path = path.unwrap_or_else(default_config_path);
    let mut config = ocluster_config::defaults::default_config();

    if let Some(v) = inference_listen {
        config.inference.listen = v;
    }
    if let Some(v) = management_listen {
        config.management.listen = v;
    }
    if let Some(v) = database_path {
        config.database.path = v;
    }

    for spec in nodes {
        let (name, url) = spec
            .split_once('@')
            .context("node spec must be name@url (e.g. gpu-01@http://127.0.0.1:11435)")?;
        config.nodes.push(ocluster_config::types::NodeConfig {
            name: name.into(),
            url: url.into(),
            model_mode: ModelMode::Discover,
            configured_models: vec![],
            max_concurrent: 8,
            priority: 0,
            labels: Default::default(),
        });
    }

    init_config(&config_path, &config).context("failed to initialise configuration")?;
    println!("Configuration written to {}", config_path.display());
    Ok(())
}

async fn cmd_serve(cli: &Cli) -> Result<()> {
    let overrides = ConfigOverrides::default();
    let config = load_config(cli.config.as_deref(), &overrides)?;
    ocluster_controller::run_controller(config)
        .await
        .context("controller exited with error")
}

async fn cmd_with_client<F, Fut>(cli: &Cli, f: F) -> Result<()>
where
    F: FnOnce(ManagementClient, OutputWriter) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let client = ManagementClient::new(&cli.endpoint)?;
    let out = OutputWriter::new(cli.output);
    f(client, out).await
}

async fn execute_node(cmd: NodeCommands, cli: &Cli) -> Result<()> {
    match cmd {
        NodeCommands::Add {
            name,
            url,
            model_mode,
        } => {
            cmd_with_client(cli, |c, out| async move {
                let resp = c
                    .add_node(&AddNodeRequest {
                        name: name.clone(),
                        url,
                        model_mode: Some(model_mode),
                        max_concurrent: None,
                    })
                    .await?;
                out.write_operation(&resp);
                Ok(())
            })
            .await
        }
        NodeCommands::Remove { name, force, yes } => {
            if !force && !yes {
                anyhow::bail!("use --yes or --force to confirm node removal");
            }
            cmd_with_client(cli, |c, out| async move {
                let resp = c.remove_node(&name).await?;
                out.write_operation(&resp);
                Ok(())
            })
            .await
        }
        NodeCommands::Enable { name } => {
            cmd_with_client(cli, |c, out| async move {
                out.write_operation(&c.enable_node(&name).await?);
                Ok(())
            })
            .await
        }
        NodeCommands::Disable { name } => {
            cmd_with_client(cli, |c, out| async move {
                out.write_operation(&c.disable_node(&name).await?);
                Ok(())
            })
            .await
        }
        NodeCommands::Drain { name } => {
            cmd_with_client(cli, |c, out| async move {
                out.write_operation(&c.drain_node(&name).await?);
                Ok(())
            })
            .await
        }
        NodeCommands::Inspect { name } => {
            cmd_with_client(cli, |c, out| async move {
                out.write_node_detail(&c.get_node(&name).await?);
                Ok(())
            })
            .await
        }
        NodeCommands::Probe { name } => {
            cmd_with_client(cli, |c, out| async move {
                let result = c.probe_node(&name).await?;
                out.write_json(&result);
                Ok(())
            })
            .await
        }
        NodeCommands::Models { command } => match command {
            NodeModelCommands::Sync { name, dry_run } => {
                if dry_run {
                    println!("dry-run: would sync models for node {name}");
                    Ok(())
                } else {
                    cmd_with_client(cli, |c, out| async move {
                        let _ = name;
                        out.write_operation(&c.sync_models().await?);
                        Ok(())
                    })
                    .await
                }
            }
        },
    }
}

async fn execute_model(cmd: ModelCommands, cli: &Cli) -> Result<()> {
    match cmd {
        ModelCommands::Inspect { name } => {
            cmd_with_client(cli, |c, out| async move {
                out.write_model_detail(&c.get_model(&name).await?);
                Ok(())
            })
            .await
        }
    }
}

async fn execute_requests(cmd: Option<RequestCommands>, cli: &Cli) -> Result<()> {
    match cmd {
        None => {
            cmd_with_client(cli, |c, out| async move {
                out.write_requests(&c.list_requests().await?);
                Ok(())
            })
            .await
        }
        Some(RequestCommands::Watch) => {
            loop {
                cmd_with_client(cli, |c, out| async move {
                    out.write_requests(&c.list_requests().await?);
                    Ok(())
                })
                .await?;
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
        Some(RequestCommands::Cancel { id }) => {
            cmd_with_client(cli, |c, out| async move {
                out.write_operation(&c.cancel_request(&id).await?);
                Ok(())
            })
            .await
        }
    }
}

async fn execute_config(cmd: ConfigCommands, cli: &Cli) -> Result<()> {
    match cmd {
        ConfigCommands::Show => {
            if let Some(path) = &cli.config {
                let config = load_config(Some(path), &ConfigOverrides::default())?;
                OutputWriter::new(cli.output).write_json(&config);
            } else {
                cmd_with_client(cli, |c, out| async move {
                    out.write_json(&c.show_config().await?);
                    Ok(())
                })
                .await?;
            }
            Ok(())
        }
        ConfigCommands::Validate => {
            if let Some(path) = &cli.config {
                let config = load_config(Some(path), &ConfigOverrides::default())?;
                OutputWriter::new(cli.output).write_json(&serde_json::json!({
                    "success": true,
                    "message": "Configuration valid",
                    "path": path.display().to_string(),
                }));
                let _ = config;
                Ok(())
            } else {
                cmd_with_client(cli, |c, out| async move {
                    out.write_operation(&c.validate_config().await?);
                    Ok(())
                })
                .await
            }
        }
        ConfigCommands::Reload => {
            cmd_with_client(cli, |c, out| async move {
                out.write_operation(&c.reload_config().await?);
                Ok(())
            })
            .await
        }
    }
}
