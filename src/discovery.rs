use std::collections::HashMap;
use std::net::IpAddr;

pub fn get_ips(hostnames: &[&str]) -> HashMap<String, IpAddr> {
    let mut ips: HashMap<String, IpAddr> = HashMap::new();
    for hostname in hostnames {
        match dns_lookup::lookup_host(hostname) {
            Ok(ip) => {
                // println!("{}: {:?}", hostname, ip);
                if let Some(ip) = ip.get(0) {
                    ips.insert(hostname.to_string(), ip.clone());
                }
            }
            Err(e) => {
                // println!("{}: {:?}", hostname, e);
            }
        }
    }
    ips
}