mod action;
mod browser;
mod cdp;
mod mcp;
mod sasp;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cdpx",
    version = env!("CARGO_PKG_VERSION"),
    about = "Driverless, token-compressed Chrome DevTools Protocol controller & stdio MCP server for AI agents",
    author = "Akash Priyadarshi"
)]
struct Cli {
    /// Start stdio Model Context Protocol (MCP) server
    #[arg(long)]
    mcp: bool,

    /// Run browser in headed mode (default is headless)
    #[arg(long)]
    headed: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Open a URL and print token-compressed interactive elements
    Open {
        /// Target URL to navigate to
        url: String,
    },
    /// Click an element reference (e.g. @e1 or 1)
    Click {
        /// Element handle (@e1, @e2)
        target: String,
        /// Optional URL to open first
        #[arg(short, long)]
        url: Option<String>,
    },
    /// Type text into an element reference
    Type {
        /// Element handle (@e1, @e2)
        target: String,
        /// Text string to type
        text: String,
        /// Optional URL to open first
        #[arg(short, long)]
        url: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.mcp {
        return mcp::run_mcp_server().await;
    }

    match cli.command {
        Some(Commands::Open { url }) => {
            println!("Launching browser and navigating to {}...", url);
            let browser = browser::launch_browser(!cli.headed).await?;
            let client = cdp::CdpClient::connect(&browser.ws_url).await?;

            client.navigate(&url).await?;
            action::wait_for_settle(&client, 1200).await?;

            let snapshot = mcp::extract_snapshot(&client).await;
            println!("{}", snapshot);
        }

        Some(Commands::Click { target, url }) => {
            let browser = browser::launch_browser(!cli.headed).await?;
            let client = cdp::CdpClient::connect(&browser.ws_url).await?;

            if let Some(u) = url {
                client.navigate(&u).await?;
                action::wait_for_settle(&client, 1200).await?;
                // Pre-populate cache
                let _ = mcp::extract_snapshot(&client).await;
            }

            println!("Clicking {}...", target);
            action::click_element(&client, &target).await?;
            let snapshot = mcp::extract_snapshot(&client).await;
            println!("{}", snapshot);
        }

        Some(Commands::Type { target, text, url }) => {
            let browser = browser::launch_browser(!cli.headed).await?;
            let client = cdp::CdpClient::connect(&browser.ws_url).await?;

            if let Some(u) = url {
                client.navigate(&u).await?;
                action::wait_for_settle(&client, 1200).await?;
                let _ = mcp::extract_snapshot(&client).await;
            }

            println!("Typing into {}...", target);
            action::type_element(&client, &target, &text).await?;
            let snapshot = mcp::extract_snapshot(&client).await;
            println!("{}", snapshot);
        }

        None => {
            println!("cdpx v{} - Driverless CDP browser controller & stdio MCP server.", env!("CARGO_PKG_VERSION"));
            println!("Run 'cdpx --help' for subcommands or 'cdpx --mcp' to start the AI agent server.");
        }
    }

    Ok(())
}
