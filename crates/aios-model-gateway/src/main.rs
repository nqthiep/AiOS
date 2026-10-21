//! `aios-model-gateway` binary: the AiOS model gateway (docs/m0/gateway.md).

use std::path::PathBuf;

use anyhow::{Context, bail};

use aios_model_gateway::config;
use aios_model_gateway::{Gateway, GatewayOptions};

/// Default main config.
const DEFAULT_CONFIG: &str = "/etc/aios/gateway.yaml";
/// Default users directory.
const DEFAULT_USERS_DIR: &str = "/var/lib/aios/users";
/// Default admin socket.
const DEFAULT_ADMIN_SOCKET: &str = "/run/aios/gateway-admin.sock";

/// Command-line arguments.
#[derive(Debug, Clone)]
struct Args {
    config: PathBuf,
    users_dir: PathBuf,
    admin_socket: PathBuf,
    admin_uid: Option<u32>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            config: PathBuf::from(DEFAULT_CONFIG),
            users_dir: PathBuf::from(DEFAULT_USERS_DIR),
            admin_socket: PathBuf::from(DEFAULT_ADMIN_SOCKET),
            admin_uid: None,
        }
    }
}

fn usage() -> String {
    format!(
        "aios-model-gateway: the AiOS model gateway\n\n\
         Usage:\n  aios-model-gateway [options]\n\n\
         Options:\n\
         \x20 --config <path>          main config (default {DEFAULT_CONFIG})\n\
         \x20 --users-dir <path>       users directory (default {DEFAULT_USERS_DIR})\n\
         \x20 --admin-socket <path>    admin socket (default {DEFAULT_ADMIN_SOCKET})\n\
         \x20 --admin-uid <uid>        the only UID allowed to call the admin API\n\
         \x20 -h, --help               print this help\n"
    )
}

/// Value of a flag (`--flag value` or `--flag=value`).
fn take_value(
    name: &str,
    inline: Option<String>,
    rest: &mut std::vec::IntoIter<String>,
) -> anyhow::Result<String> {
    match inline {
        Some(value) => Ok(value),
        None => rest.next().with_context(|| format!("{name} needs a value")),
    }
}

/// Parse command-line arguments.
fn parse_args(raw: Vec<String>) -> anyhow::Result<Args> {
    let mut args = Args::default();
    let mut rest = raw.into_iter();
    while let Some(argument) = rest.next() {
        let (name, inline) = match argument.split_once('=') {
            Some((name, value)) => (name.to_string(), Some(value.to_string())),
            None => (argument.clone(), None),
        };
        match name.as_str() {
            "--config" => {
                args.config = PathBuf::from(take_value("--config", inline, &mut rest)?);
            }
            "--users-dir" => {
                args.users_dir = PathBuf::from(take_value("--users-dir", inline, &mut rest)?);
            }
            "--admin-socket" => {
                args.admin_socket = PathBuf::from(take_value("--admin-socket", inline, &mut rest)?);
            }
            "--admin-uid" => {
                let value = take_value("--admin-uid", inline, &mut rest)?;
                args.admin_uid = Some(
                    value
                        .parse::<u32>()
                        .with_context(|| format!("invalid --admin-uid '{value}'"))?,
                );
            }
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            other => bail!("unknown argument '{other}'\n\n{}", usage()),
        }
    }
    Ok(args)
}

/// Enable JSON `tracing` logs.
fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = parse_args(std::env::args().skip(1).collect())?;
    init_tracing();

    let config = config::load_gateway_config(&args.config)
        .with_context(|| format!("cannot load {}", args.config.display()))?;

    // Without `--admin-uid` the admin API stays closed (fail closed).
    let admin_socket = if args.admin_uid.is_some() {
        Some(args.admin_socket.clone())
    } else {
        tracing::warn!(
            socket = %args.admin_socket.display(),
            "no --admin-uid given: the admin API stays disabled"
        );
        None
    };

    let gateway = Gateway::bind(GatewayOptions {
        config,
        users_dir: args.users_dir.clone(),
        admin_socket,
        admin_uid: args.admin_uid,
    })
    .await?;

    let address = gateway.api_addr()?;
    tracing::info!(
        %address,
        users_dir = %args.users_dir.display(),
        admin_socket = %args.admin_socket.display(),
        "aios-model-gateway started"
    );

    gateway
        .serve_until(async {
            if let Err(error) = tokio::signal::ctrl_c().await {
                tracing::error!(error = %error, "cannot listen for Ctrl-C");
            }
        })
        .await
}
