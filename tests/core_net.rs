use spin_up::core_net::{IpAddr, IpAddrV4, IpAddrV6, SocketAddr, SocketAddrV4, SocketAddrV6};

#[test]
fn ip_addr_v4_is_constructable() {
    let addr = IpAddrV4 {
        octets: vec![127.0, 0.0, 0.0, 1.0],
    };
    assert_eq!(addr.octets, vec![127.0, 0.0, 0.0, 1.0]);
}

#[test]
fn ip_addr_v6_is_constructable() {
    let addr = IpAddrV6 {
        octets: vec![0.0; 16],
    };
    assert_eq!(addr.octets.len(), 16);
}

#[test]
fn ip_addr_choice_v4_variant() {
    let v4 = IpAddrV4 {
        octets: vec![10.0, 0.0, 0.0, 1.0],
    };
    let addr = IpAddr::V4(v4);
    assert!(matches!(addr, IpAddr::V4(ref inner) if inner.octets == vec![10.0, 0.0, 0.0, 1.0]));
}

#[test]
fn ip_addr_choice_v6_variant() {
    let v6 = IpAddrV6 {
        octets: vec![0.0; 16],
    };
    let addr = IpAddr::V6(v6);
    assert!(matches!(addr, IpAddr::V6(_)));
}

#[test]
fn socket_addr_v4_is_constructable() {
    let sock = SocketAddrV4 {
        ip: IpAddrV4 {
            octets: vec![192.0, 168.0, 1.0, 1.0],
        },
        port: 8080.0,
    };
    assert_eq!(sock.port, 8080.0);
    assert_eq!(sock.ip.octets, vec![192.0, 168.0, 1.0, 1.0]);
}

#[test]
fn socket_addr_v6_is_constructable() {
    let sock = SocketAddrV6 {
        ip: IpAddrV6 {
            octets: vec![0.0; 16],
        },
        port: 443.0,
    };
    assert_eq!(sock.port, 443.0);
}

#[test]
fn socket_addr_choice_v4_variant() {
    let sock = SocketAddr::V4(SocketAddrV4 {
        ip: IpAddrV4 {
            octets: vec![127.0, 0.0, 0.0, 1.0],
        },
        port: 3000.0,
    });
    assert!(matches!(sock, SocketAddr::V4(ref inner) if inner.port == 3000.0));
}

#[test]
fn socket_addr_choice_v6_variant() {
    let sock = SocketAddr::V6(SocketAddrV6 {
        ip: IpAddrV6 {
            octets: vec![0.0; 16],
        },
        port: 9090.0,
    });
    assert!(matches!(sock, SocketAddr::V6(ref inner) if inner.port == 9090.0));
}
