use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

mod chat;
mod status;

/// NDE-OS — AI Operating System CLI
#[derive(Parser)]
#[command(
    name = "nde",
    version = "0.2.0",
    about = "NDE-OS AI Operating System CLI",
    long_about = "Manage apps, agents, plugins, and models from your terminal."
)]
struct Cli {
    /// API server URL
    #[arg(long, default_value = "http://localhost:8080", env = "NDE_API_URL")]
    api_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Chat with the AI agent
    Chat {
        /// Message to send (omit for interactive REPL)
        message: Option<String>,

        /// Enable streaming mode
        #[arg(long, short)]
        stream: bool,

        /// Conversation ID to continue
        #[arg(long)]
        conversation: Option<String>,

        /// LLM provider to use
        #[arg(long)]
        provider: Option<String>,
    },

    /// Manage applications
    App {
        #[command(subcommand)]
        action: AppAction,
    },

    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
    },

    /// Manage LLM models/providers
    Model {
        #[command(subcommand)]
        action: ModelAction,
    },

    /// Show system status
    Status,

    /// Start the MCP server (expose tools to Claude Code, etc.)
    Mcp,
}

#[derive(Subcommand)]
enum AppAction {
    /// List installed apps
    List,
    /// Show catalog of available apps
    Catalog,
    /// Install an app by ID
    Install {
        /// App manifest JSON or ID from catalog
        manifest: String,
    },
    /// Launch a running app
    Launch {
        /// App ID to launch
        id: String,
    },
    /// Stop a running app
    Stop {
        /// App ID to stop
        id: String,
    },
    /// Uninstall an app
    Uninstall {
        /// App ID to uninstall
        id: String,
    },
    /// Show app info
    Info {
        /// App ID
        id: String,
    },
}

#[derive(Subcommand)]
enum PluginAction {
    /// List all plugins
    List,
    /// Install a plugin
    Install {
        /// Plugin ID
        id: String,
    },
    /// Start a plugin
    Start {
        /// Plugin ID
        id: String,
    },
    /// Stop a plugin
    Stop {
        /// Plugin ID
        id: String,
    },
}

#[derive(Subcommand)]
enum ModelAction {
    /// List available LLM providers
    List,
    /// Show active provider
    Active,
    /// Switch active provider
    Switch {
        /// Provider name
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();
    let api = &cli.api_url;

    match cli.command {
        Commands::Chat {
            message,
            stream,
            conversation,
            provider,
        } => {
            if let Some(msg) = message {
                chat::send_message(&client, api, &msg, stream, conversation.as_deref(), provider.as_deref()).await?;
            } else {
                chat::interactive_repl(&client, api, stream, provider.as_deref()).await?;
            }
        }

        Commands::App { action } => match action {
            AppAction::List => {
                let resp: serde_json::Value = client
                    .get(format!("{}/api/apps", api))
                    .send()
                    .await?
                    .json()
                    .await?;
                println!("{}", serde_json::to_string_pretty(&resp["data"])?);
            }
            AppAction::Catalog => {
                let resp: serde_json::Value = client
                    .get(format!("{}/api/catalog", api))
                    .send()
                    .await?
                    .json()
                    .await?;
                if let Some(apps) = resp["data"].as_array() {
                    for app in apps {
                        println!(
                            "  {} — {}",
                            app["id"].as_str().unwrap_or("?").green(),
                            app["description"].as_str().unwrap_or("")
                        );
                    }
                }
            }
            AppAction::Install { manifest } => {
                let body = serde_json::json!({"manifest": serde_json::from_str::<serde_json::Value>(&manifest)?});
                let resp: serde_json::Value = client
                    .post(format!("{}/api/apps", api))
                    .json(&body)
                    .send()
                    .await?
                    .json()
                    .await?;
                println!("{}", resp["message"].as_str().unwrap_or("Done"));
            }
            AppAction::Launch { id } => {
                let resp: serde_json::Value = client
                    .post(format!("{}/api/apps/{}/launch", api, id))
                    .send()
                    .await?
                    .json()
                    .await?;
                if resp["success"].as_bool() == Some(true) {
                    println!(
                        "{} launched — PID: {}, Port: {}",
                        id.green(),
                        resp["data"]["pid"],
                        resp["data"]["port"]
                    );
                } else {
                    eprintln!("{}: {}", "Error".red(), resp["message"]);
                }
            }
            AppAction::Stop { id } => {
                let resp: serde_json::Value = client
                    .post(format!("{}/api/apps/{}/stop", api, id))
                    .send()
                    .await?
                    .json()
                    .await?;
                println!("{}", resp["message"].as_str().unwrap_or("Done"));
            }
            AppAction::Uninstall { id } => {
                let resp: serde_json::Value = client
                    .delete(format!("{}/api/apps/{}", api, id))
                    .send()
                    .await?
                    .json()
                    .await?;
                println!("{}", resp["message"].as_str().unwrap_or("Done"));
            }
            AppAction::Info { id } => {
                let resp: serde_json::Value = client
                    .get(format!("{}/api/apps/{}", api, id))
                    .send()
                    .await?
                    .json()
                    .await?;
                println!("{}", serde_json::to_string_pretty(&resp["data"])?);
            }
        },

        Commands::Plugin { action } => match action {
            PluginAction::List => {
                let resp: serde_json::Value = client
                    .get(format!("{}/api/plugins", api))
                    .send()
                    .await?
                    .json()
                    .await?;
                if let Some(plugins) = resp["data"].as_array() {
                    for p in plugins {
                        let state = p["state"].as_str().unwrap_or("?");
                        let icon = match state {
                            "running" => "●".green(),
                            "installed" => "○".yellow(),
                            _ => "○".dimmed(),
                        };
                        println!(
                            "  {} {} v{} [{}]",
                            icon,
                            p["name"].as_str().unwrap_or("?").bold(),
                            p["version"].as_str().unwrap_or("?"),
                            state
                        );
                    }
                }
            }
            PluginAction::Install { id } => {
                let resp: serde_json::Value = client
                    .post(format!("{}/api/plugins/{}/install", api, id))
                    .send()
                    .await?
                    .json()
                    .await?;
                println!("{}", resp["message"].as_str().unwrap_or("Done"));
            }
            PluginAction::Start { id } => {
                let resp: serde_json::Value = client
                    .post(format!("{}/api/plugins/{}/start", api, id))
                    .send()
                    .await?
                    .json()
                    .await?;
                println!("{}", resp["message"].as_str().unwrap_or("Done"));
            }
            PluginAction::Stop { id } => {
                let resp: serde_json::Value = client
                    .post(format!("{}/api/plugins/{}/stop", api, id))
                    .send()
                    .await?
                    .json()
                    .await?;
                println!("{}", resp["message"].as_str().unwrap_or("Done"));
            }
        },

        Commands::Model { action } => match action {
            ModelAction::List => {
                let resp: serde_json::Value = client
                    .get(format!("{}/api/models", api))
                    .send()
                    .await?
                    .json()
                    .await?;
                if let Some(models) = resp["data"].as_array() {
                    for m in models {
                        let active = if m["is_active"].as_bool() == Some(true) {
                            " ← active".green()
                        } else {
                            "".normal()
                        };
                        println!(
                            "  {} [{}]{}",
                            m["name"].as_str().unwrap_or("?").bold(),
                            m["provider_type"].as_str().unwrap_or("?"),
                            active
                        );
                    }
                }
            }
            ModelAction::Active => {
                let resp: serde_json::Value = client
                    .get(format!("{}/api/models/active", api))
                    .send()
                    .await?
                    .json()
                    .await?;
                println!("Active: {}", resp["data"].as_str().unwrap_or("none").green());
            }
            ModelAction::Switch { name } => {
                let resp: serde_json::Value = client
                    .post(format!("{}/api/models/switch", api))
                    .json(&serde_json::json!({"name": name}))
                    .send()
                    .await?
                    .json()
                    .await?;
                println!("{}", resp["message"].as_str().unwrap_or("Done"));
            }
        },

        Commands::Status => {
            status::show_status(&client, api).await?;
        }

        Commands::Mcp => {
            println!("{}", "Starting NDE-OS MCP server on stdio...".bold());
            let server = ai_launcher_core::mcp::server::McpServer::new();
            server.run_stdio().await?;
        }
    }

    Ok(())
}
