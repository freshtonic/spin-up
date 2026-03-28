#[lang-item]
record IpAddrV4 {
  octets: [u8; 4],
}

#[lang-item]
record IpAddrV6 {
  octets: [u8; 16],
}

#[lang-item]
choice IpAddr {
  V4(IpAddrV4),
  V6(IpAddrV6),
}

#[lang-item]
record SocketAddrV4 {
  ip: IpAddrV4,
  port: u16,
}

#[lang-item]
record SocketAddrV6 {
  ip: IpAddrV6,
  port: u16,
}

#[lang-item]
choice SocketAddr {
  V4(SocketAddrV4),
  V6(SocketAddrV6),
}
