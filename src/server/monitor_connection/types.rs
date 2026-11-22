//! Plain types of the ngrok connection: secrets, environment, connector
//! interface, the API snapshot and the error enums.

use std::any::Any;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::Serialize;
use url::Url;

pub use super::connector::{RoutingConnector, SdkConnector};
use super::contracts::{ConnectionStatus, Provider, PublicOrigin};
pub use super::input::{parse_domain, parse_token};
use super::keychain::{NoKeychain, OsKeychain, TokenVault};
use crate::server::monitor_auth::passcode::Passcode;

/// A value that must never be printed.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

/// Values taken from the process environment at startup.
#[derive(Clone, Default)]
pub struct MonitorEnv {
    /// `NGROK_AUTHTOKEN`.
    pub token: Option<Secret>,
    /// `AGBOARD_NGROK_DOMAIN`: the ngrok domain to open the endpoint on, so the public
    /// address is the same at every start. Anything that is not a bare host name is ignored.
    pub domain: Option<String>,
}

impl MonitorEnv {
    pub fn from_process() -> Self {
        let get = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        Self {
            token: get("NGROK_AUTHTOKEN").map(Secret::new),
            domain: get("AGBOARD_NGROK_DOMAIN").and_then(|v| parse_domain(&v)),
        }
    }
}

/// Short, fixed reasons a start can fail. Safe to show and to log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    ConnectFailed,
    EndpointFailed,
    PublicUrlInvalid,
    Internal,
    ConfigUnreadable,
    ConfigInvalid,
    /// Tailscale is chosen but its command-line tool was not found.
    TailscaleNotInstalled,
    /// Tailscale is installed but not running or not logged in.
    TailscaleNotLoggedIn,
    /// HTTPS certificates are not enabled for the tailnet, so there is no `ts.net` address.
    TailscaleHttpsDisabled,
    /// `tailscale serve` did not accept the entrance.
    TailscaleServeFailed,
    /// Another program already serves HTTPS on port 443 of this tailnet name.
    TailscaleServeBusy,
}

impl FailureCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConnectFailed => "connect_failed",
            Self::EndpointFailed => "endpoint_failed",
            Self::PublicUrlInvalid => "public_url_invalid",
            Self::Internal => "internal",
            Self::ConfigUnreadable => "config_unreadable",
            Self::ConfigInvalid => "config_invalid",
            Self::TailscaleNotInstalled => "tailscale_not_installed",
            Self::TailscaleNotLoggedIn => "tailscale_not_logged_in",
            Self::TailscaleHttpsDisabled => "tailscale_https_disabled",
            Self::TailscaleServeFailed => "tailscale_serve_failed",
            Self::TailscaleServeBusy => "tailscale_serve_busy",
        }
    }
}

/// What the connector needs. `token` is a secret and is empty for Tailscale, which needs none.
/// There is no Traffic Policy. Without a `domain` ngrok chooses the public address, and it
/// differs at every start; Tailscale ignores `domain`.
pub struct OpenRequest {
    pub provider: Provider,
    pub token: Secret,
    pub domain: Option<String>,
    pub backend: Url,
}

/// An open tunnel. Dropping `keep_alive` closes it.
pub struct OpenedTunnel {
    pub public: PublicOrigin,
    pub keep_alive: Box<dyn Any + Send>,
}

#[derive(Debug)]
pub struct OpenFailure {
    pub code: FailureCode,
}

pub type OpenFuture = Pin<Box<dyn Future<Output = Result<OpenedTunnel, OpenFailure>> + Send>>;

/// Opens the public endpoint. The real one uses the ngrok SDK; tests use a mock
/// so that nothing reaches the network.
pub trait Connector: Send + Sync {
    fn open(&self, request: OpenRequest) -> OpenFuture;
}

/// How the built-in monitor connection is set up for one run.
#[derive(Clone)]
pub struct MonitorOptions {
    /// Values from the process environment (the ngrok token).
    pub env: MonitorEnv,
    /// `--monitor ngrok` was given on the command line. The run must then start the
    /// connection before it announces itself and fails if it cannot.
    pub explicit: bool,
    /// Opens the public endpoint. Tests replace it so nothing reaches the network.
    pub connector: Arc<dyn Connector>,
    /// Where a saved token lives. Tests replace it so the real keychain is untouched.
    pub vault: Arc<dyn TokenVault>,
    /// Posts Web Push messages to the push services. Tests replace it so nothing reaches one.
    pub push_transport: Arc<dyn crate::server::push_delivery::PushTransport>,
}

impl MonitorOptions {
    /// Real SDK connector, the OS keychain and the given environment.
    pub fn new(env: MonitorEnv, explicit: bool) -> Self {
        Self {
            env,
            explicit,
            connector: Arc::new(RoutingConnector::new()),
            vault: Arc::new(OsKeychain),
            push_transport: Arc::new(crate::server::push_delivery::HttpTransport::new()),
        }
    }

    /// Like [`MonitorOptions::new`] but with no keychain: nothing is read or saved.
    pub fn without_keychain(env: MonitorEnv, explicit: bool) -> Self {
        Self {
            vault: Arc::new(NoKeychain),
            ..Self::new(env, explicit)
        }
    }
}

/// What a monitor started on the command line reports once it is ready: the
/// public URL and the passcode of this start, for the PC to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorReady {
    pub url: String,
    pub passcode: Passcode,
}

/// What `GET /api/monitor-connections` returns: nothing secret.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub status: ConnectionStatus,
    pub url: Option<String>,
    pub revision: u64,
    pub settings: Settings,
    /// The domain from `AGBOARD_NGROK_DOMAIN`. When set it is the one used, over the saved one.
    pub domain_env: Option<String>,
    pub token_configured: bool,
    /// Which token a start would use; `None` when there is none. Never the value.
    pub token_source: Option<TokenSource>,
    /// Whether a token can be saved in the OS keychain here.
    pub keychain_available: bool,
    /// Whether a token is saved in the keychain (even if another one wins).
    pub keychain_token_saved: bool,
    pub failure: Option<FailureCode>,
    /// The provider of the connection that is starting, ready or reconnecting; `null` when none
    /// is running. It can differ from `settings.provider`, which is only what is saved.
    pub active_provider: Option<Provider>,
    /// The passcode of the running connection, for the PC to show. Present only
    /// while the status is `ready`; a stopped or failed connection has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passcode: Option<Passcode>,
}

/// Where the token in use came from.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenSource {
    /// `NGROK_AUTHTOKEN`.
    Env,
    /// The OS keychain.
    Keychain,
    /// Typed on the PC for this run only.
    Session,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub enabled: bool,
    pub provider: Provider,
    /// The saved domain; `None` when none is saved.
    pub domain: Option<String>,
}

/// A settings change from the PC. `revision` is the one the PC last saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsUpdate {
    pub enabled: bool,
    pub revision: u64,
    /// `None` keeps the saved provider.
    pub provider: Option<Provider>,
    /// `None` keeps the saved domain, `Some(None)` clears it, `Some(Some(d))` saves `d`.
    pub domain: Option<Option<String>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateError {
    Conflict,
    SaveFailed,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StartError {
    /// ngrok is chosen, no token is held and none was given.
    TokenRequired,
    /// The saved settings file could not be read; save the settings first.
    SettingsUnreadable,
    /// The enabled setting could not be saved, so nothing was started.
    SaveFailed,
    /// Saving the token was asked for but the keychain cannot take it; nothing was started.
    KeychainUnavailable,
    Internal,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ForgetError {
    /// The keychain is not usable, so the saved token cannot be removed.
    KeychainUnavailable,
}
