#[lang-item]
type IpAddrV4 =
  octets: [u8; 4],
;

#[lang-item]
type IpAddrV6 =
  octets: [u8; 16],
;

#[lang-item]
type IpAddr = V4(IpAddrV4) | V6(IpAddrV6);

#[lang-item]
type SocketAddrV4 =
  ip: IpAddrV4,
  port: u16,
;

#[lang-item]
type SocketAddrV6 =
  ip: IpAddrV6,
  port: u16,
;

#[lang-item]
type SocketAddr = V4(SocketAddrV4) | V6(SocketAddrV6);
