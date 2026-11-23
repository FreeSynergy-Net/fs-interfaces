//! The address rule for feeds and accounts (R7/K4): which addresses a fetch may go to.
//!
//! Pattern: Specification (one pure check; the fake "resolves" names through a table, a real provider connects to
//! exactly the checked address and checks every redirect again).

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::draft::{BasicValue, Reference};

/// `None` when the address may be fetched, else the code of the problem (never the address itself).
#[must_use]
pub fn refusal<S: std::hash::BuildHasher>(
    url: &Reference,
    dns: &HashMap<String, IpAddr, S>,
) -> Option<&'static str> {
    let text = url.canonical();
    let Some((scheme, rest)) = text.split_once(':') else {
        return Some("url.scheme");
    };
    if !["https", "webcal"].contains(&scheme.to_ascii_lowercase().as_str()) {
        return Some("url.scheme");
    }
    let Some(rest) = rest.strip_prefix("//") else {
        return Some("url.host");
    };
    let auth = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if auth.contains('@') {
        return Some("url.credentials");
    }
    let host = match auth.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or_default(),
        None => auth.split(':').next().unwrap_or_default(),
    };
    if host.is_empty() {
        return Some("url.host");
    }
    let lower = host.to_ascii_lowercase();
    let named = lower == "localhost"
        || [".localhost", ".local", ".internal", ".lan"]
            .iter()
            .any(|s| lower.ends_with(s));
    let ip = lower
        .parse::<IpAddr>()
        .ok()
        .or_else(|| dns.get(&lower).copied());
    if named || ip.is_some_and(|ip| !global(ip)) {
        return Some("url.private");
    }
    None
}

fn global(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => global4(a),
        IpAddr::V6(a) => global6(a),
    }
}

fn global4(a: Ipv4Addr) -> bool {
    let o = a.octets();
    !(o[0] == 0
        || o[0] == 10
        || (o[0] == 100 && (64..128).contains(&o[1]))
        || o[0] == 127
        || (o[0] == 169 && o[1] == 254)
        || (o[0] == 172 && (16..32).contains(&o[1]))
        || (o[0] == 192 && o[1] == 0 && o[2] <= 2)
        || (o[0] == 192 && o[1] == 168)
        || (o[0] == 198 && (o[1] == 18 || o[1] == 19))
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)
        || o[0] >= 224)
}

fn embedded(hi: u16, lo: u16) -> Ipv4Addr {
    let [a, b] = hi.to_be_bytes();
    let [c, d] = lo.to_be_bytes();
    Ipv4Addr::new(a, b, c, d)
}

fn global6(a: Ipv6Addr) -> bool {
    let s = a.segments();
    if let Some(v4) = a.to_ipv4_mapped() {
        return global4(v4);
    }
    if a.is_unspecified() || a.is_loopback() {
        return false;
    }
    if s[0] & 0xfe00 == 0xfc00 || s[0] & 0xffc0 == 0xfe80 || s[0] & 0xff00 == 0xff00 {
        return false;
    }
    if s[0] == 0x2001 && (s[1] == 0 || s[1] == 0x0db8) {
        return false;
    }
    if s[0] == 0x64 && s[1] == 0xff9b && s[2..6].iter().all(|x| *x == 0) {
        return global4(embedded(s[6], s[7]));
    }
    if s[0] == 0x2002 {
        return global4(embedded(s[1], s[2]));
    }
    if s[..6].iter().all(|x| *x == 0) {
        return global4(embedded(s[6], s[7]));
    }
    true
}
