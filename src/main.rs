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
        /// Target URL to open first
        #[arg(short, long)]
        url: Option<String>,
    },
    /// Type text into an element reference
    Type {
        /// Element handle (@e1, @e2)
        target: String,
        /// Text string to type
        text: String,
        /// Target URL to open first
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
            action::wait_for_settle(&client, 1500).await?;

            let snapshot = mcp::extract_snapshot(&client).await;
            println!("{}", snapshot);
        }

        Some(Commands::Click { target, url }) => {
            let target_url = match url {
                Some(u) => u,
                None => {
                    eprintln!("Error: --url <URL> is required for standalone CLI action. For interactive agent sessions, run 'cdpx --mcp'.");
                    std::process::exit(1);
                }
            };

            let browser = browser::launch_browser(!cli.headed).await?;
            let client = cdp::CdpClient::connect(&browser.ws_url).await?;

            client.navigate(&target_url).await?;
            action::wait_for_settle(&client, 1500).await?;
            let _ = mcp::extract_snapshot(&client).await;

            println!("Clicking {} on {}...", target, target_url);
            action::click_element(&client, &target).await?;
            action::wait_for_settle(&client, 800).await?;

            let snapshot = mcp::extract_snapshot(&client).await;
            println!("{}", snapshot);
        }

        Some(Commands::Type { target, text, url }) => {
            let target_url = match url {
                Some(u) => u,
                None => {
                    eprintln!("Error: --url <URL> is required for standalone CLI action. For interactive agent sessions, run 'cdpx --mcp'.");
                    std::process::exit(1);
                }
            };

            let browser = browser::launch_browser(!cli.headed).await?;
            let client = cdp::CdpClient::connect(&browser.ws_url).await?;

            client.navigate(&target_url).await?;
            action::wait_for_settle(&client, 1500).await?;
            let _ = mcp::extract_snapshot(&client).await;

            println!("Typing into {} on {}...", target, target_url);
            action::type_element(&client, &target, &text).await?;
            action::wait_for_settle(&client, 800).await?;

            let snapshot = mcp::extract_snapshot(&client).await;
            println!("{}", snapshot);
        }

        None => {
            println!("cdpx v{} - Driverless CDP Browser Controller & MCP Server", env!("CARGO_PKG_VERSION"));
            println!("Run 'cdpx --help' for usage, or 'cdpx --mcp' to launch MCP server.");
        }
    }

    Ok(())
}
