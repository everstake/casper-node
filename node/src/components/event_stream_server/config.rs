use datasize::DataSize;
use serde::{Deserialize, Serialize};

/// Default binding address for the SSE HTTP server.
///
/// Uses a fixed port per node, but binds on any interface.
const DEFAULT_ADDRESS: &str = "0.0.0.0:9999";

/// Default number of SSEs to buffer.
const DEFAULT_EVENT_STREAM_BUFFER_LENGTH: u32 = 100;

/// SSE HTTP server configuration.
#[derive(Clone, DataSize, Debug, Deserialize, Serialize)]
// Disallow unknown fields to ensure config files and command-line overrides contain valid keys.
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Address to bind event stream SSE HTTP server to.
    pub address: String,

    /// Number of SSEs to buffer.
    pub event_stream_buffer_length: u32,
}

impl Config {
    /// Creates a default instance for `EventStreamServer`.
    pub fn new() -> Self {
        Config {
            address: DEFAULT_ADDRESS.to_string(),
            event_stream_buffer_length: DEFAULT_EVENT_STREAM_BUFFER_LENGTH,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}
