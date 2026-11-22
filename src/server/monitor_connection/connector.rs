//! The connectors that open the public endpoint: the ngrok SDK, Tailscale Serve, and the one
//! that picks between them by the saved provider.

use super::contracts::{EndpointError, Provider, open_endpoint};
use super::types::{Connector, FailureCode, OpenFailure, OpenFuture, OpenRequest, OpenedTunnel};

pub struct SdkConnector;

impl Connector for SdkConnector {
    fn open(&self, request: OpenRequest) -> OpenFuture {
        Box::pin(async move {
            let endpoint = open_endpoint(
                request.token.expose(),
                request.backend,
                request.domain.as_deref(),
                None,
            )
            .await
            .map_err(|e| {
                // Diagnostic only: the reason ngrok gave. The error text never holds the token.
                eprintln!("agboard: ngrok did not open the tunnel: {e}");
                OpenFailure {
                    code: match e {
                        EndpointError::Connect(_) => FailureCode::ConnectFailed,
                        EndpointError::Listen(_) => FailureCode::EndpointFailed,
                        EndpointError::PublicUrl => FailureCode::PublicUrlInvalid,
                    },
                }
            })?;
            Ok(OpenedTunnel {
                public: endpoint.public.clone(),
                keep_alive: Box::new(endpoint),
            })
        })
    }
}

/// The real connector: ngrok or Tailscale, whichever the request names.
pub struct RoutingConnector {
    ngrok: SdkConnector,
    tailscale: super::tailscale::TailscaleConnector,
}

impl RoutingConnector {
    pub fn new() -> Self {
        Self {
            ngrok: SdkConnector,
            tailscale: super::tailscale::TailscaleConnector::new(),
        }
    }
}

impl Default for RoutingConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl Connector for RoutingConnector {
    fn open(&self, request: OpenRequest) -> OpenFuture {
        match request.provider {
            Provider::Ngrok => self.ngrok.open(request),
            Provider::Tailscale => self.tailscale.open(request),
        }
    }
}
