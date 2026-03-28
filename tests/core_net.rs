use spin_up::core_net::{IpAddr, IpAddrV4, IpAddrV6, SocketAddr, SocketAddrV4, SocketAddrV6};

#[test]
fn ip_addr_v4_is_constructable() {
    let addr = IpAddrV4 {
        octets: [127, 0, 0, 1],
    };
    assert_eq!(addr.octets, [127, 0, 0, 1]);
}

#[test]
fn ip_addr_v6_is_constructable() {
    let addr = IpAddrV6 {
        octets: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    };
    assert_eq!(addr.octets[15], 1);
}

#[test]
fn ip_addr_choice_v4_variant() {
    let v4 = IpAddrV4 {
        octets: [10, 0, 0, 1],
    };
    let addr = IpAddr::V4(v4);
    assert!(matches!(addr, IpAddr::V4(ref inner) if inner.octets == [10, 0, 0, 1]));
}

#[test]
fn ip_addr_choice_v6_variant() {
    let v6 = IpAddrV6 { octets: [0; 16] };
    let addr = IpAddr::V6(v6);
    assert!(matches!(addr, IpAddr::V6(_)));
}

#[test]
fn socket_addr_v4_is_constructable() {
    let sock = SocketAddrV4 {
        ip: IpAddrV4 {
            octets: [192, 168, 1, 1],
        },
        port: 8080,
    };
    assert_eq!(sock.port, 8080);
    assert_eq!(sock.ip.octets, [192, 168, 1, 1]);
}

#[test]
fn socket_addr_v6_is_constructable() {
    let sock = SocketAddrV6 {
        ip: IpAddrV6 { octets: [0; 16] },
        port: 443,
    };
    assert_eq!(sock.port, 443);
}

#[test]
fn socket_addr_choice_v4_variant() {
    let sock = SocketAddr::V4(SocketAddrV4 {
        ip: IpAddrV4 {
            octets: [127, 0, 0, 1],
        },
        port: 3000,
    });
    assert!(matches!(sock, SocketAddr::V4(ref inner) if inner.port == 3000));
}

#[test]
fn socket_addr_choice_v6_variant() {
    let sock = SocketAddr::V6(SocketAddrV6 {
        ip: IpAddrV6 { octets: [0; 16] },
        port: 9090,
    });
    assert!(matches!(sock, SocketAddr::V6(ref inner) if inner.port == 9090));
}
