//! `IpAddr` Type.

/// IP Address.
pub(crate) struct IpAddr(std::net::IpAddr);

impl From<IpAddr> for std::net::IpAddr {
    fn from(value: IpAddr) -> Self {
        value.0
    }
}
