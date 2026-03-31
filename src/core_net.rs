use spin_core_macros::spin_core;

#[spin_core(module = "spin-core-net", resource = "IpAddrV4")]
pub struct IpAddrV4 {
    pub octets: Vec<f64>,
}

#[spin_core(module = "spin-core-net", resource = "IpAddrV6")]
pub struct IpAddrV6 {
    pub octets: Vec<f64>,
}

#[spin_core(module = "spin-core-net", resource = "IpAddr")]
pub enum IpAddr {
    V4(IpAddrV4),
    V6(IpAddrV6),
}

#[spin_core(module = "spin-core-net", resource = "SocketAddrV4")]
pub struct SocketAddrV4 {
    pub ip: IpAddrV4,
    pub port: f64,
}

#[spin_core(module = "spin-core-net", resource = "SocketAddrV6")]
pub struct SocketAddrV6 {
    pub ip: IpAddrV6,
    pub port: f64,
}

#[spin_core(module = "spin-core-net", resource = "SocketAddr")]
pub enum SocketAddr {
    V4(SocketAddrV4),
    V6(SocketAddrV6),
}
