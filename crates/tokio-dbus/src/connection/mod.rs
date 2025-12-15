#[cfg(feature = "tokio")]
#[doc(inline)]
use self::transport::Transport;
#[cfg(feature = "tokio")]
pub(crate) use self::transport::{TransportState, sasl_recv};
#[cfg(feature = "tokio")]
mod transport;

#[cfg(feature = "tokio")]
pub use self::builder::ConnectionBuilder;
#[cfg(feature = "tokio")]
mod builder;

#[cfg(feature = "tokio")]
pub use self::connection::Connection;
#[cfg(feature = "tokio")]
mod connection;

#[cfg(feature = "alloc")]
mod buffers;
#[cfg(feature = "alloc")]
pub use self::buffers::Buffers;
