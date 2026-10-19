use std::net::IpAddr;
use std::str::FromStr;

use ipnet::IpNet;

const V4_PUBLIC: &str = "100.64.0.0/10";

const V4_PRIVATE: &[&str] = &[
    "0.0.0.0/8",
    "10.0.0.0/8",
    "127.0.0.0/8",
    "169.254.0.0/16",
    "172.16.0.0/12",
    "192.0.0.0/24",
    "192.0.0.170/31",
    "192.0.2.0/24",
    "192.168.0.0/16",
    "198.18.0.0/15",
    "198.51.100.0/24",
    "203.0.113.0/24",
    "240.0.0.0/4",
    "255.255.255.255/32",
];

const V4_PRIVATE_EXCEPTIONS: &[&str] = &["192.0.0.9/32", "192.0.0.10/32"];

const V6_PRIVATE: &[&str] = &[
    "::1/128",
    "::/128",
    "::ffff:0:0/96",
    "64:ff9b:1::/48",
    "100::/64",
    "2001::/23",
    "2001:db8::/32",
    "2002::/16",
    "3fff::/20",
    "fc00::/7",
    "fe80::/10",
];

const V6_PRIVATE_EXCEPTIONS: &[&str] = &[
    "2001:1::1/128",
    "2001:1::2/128",
    "2001:3::/32",
    "2001:4:112::/48",
    "2001:20::/28",
    "2001:30::/28",
];

fn parse_net(s: &str) -> IpNet {
    IpNet::from_str(s).expect("CIDR constante invalida")
}

fn in_any(ip: IpAddr, nets: &[&str]) -> bool {
    nets.iter().any(|n| parse_net(n).contains(&ip))
}

fn is_private_v4(ip: IpAddr) -> bool {
    in_any(ip, V4_PRIVATE) && !in_any(ip, V4_PRIVATE_EXCEPTIONS)
}

fn is_private_v6(ip: IpAddr) -> bool {
    in_any(ip, V6_PRIVATE) && !in_any(ip, V6_PRIVATE_EXCEPTIONS)
}

/// Replica exacta de Python 3.12 ipaddress.*.is_global.
pub fn is_global(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let ip = IpAddr::V4(v4);
            !parse_net(V4_PUBLIC).contains(&ip) && !is_private_v4(ip)
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_global(IpAddr::V4(mapped));
            }
            !is_private_v6(ip)
        }
    }
}
