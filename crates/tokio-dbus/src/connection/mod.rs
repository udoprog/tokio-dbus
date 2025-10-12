#[doc(inline)]
use self::transport::Transport;
pub(crate) use self::transport::{TransportState, sasl_recv};
mod transport;

pub use self::builder::ConnectionBuilder;
mod builder;

pub use self::connection::Connection;
mod connection;
