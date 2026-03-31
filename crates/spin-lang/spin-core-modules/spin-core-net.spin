#[lang-item]
type IpAddrV4 =
  octets: [number],
;

#[lang-item]
type IpAddrV6 =
  octets: [number],
;

#[lang-item]
type IpAddr = V4(IpAddrV4) | V6(IpAddrV6);

#[lang-item]
type SocketAddrV4 =
  ip: IpAddrV4,
  port: number,
;

#[lang-item]
type SocketAddrV6 =
  ip: IpAddrV6,
  port: number,
;

#[lang-item]
type SocketAddr = V4(SocketAddrV4) | V6(SocketAddrV6);
