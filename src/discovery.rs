use std::collections::HashMap;

pub fn get_ips(hostnames: &[&str]) -> HashMap<String, Result<String, String>> {
    let mut ips: HashMap<String, Result<String, String>> = HashMap::new();
    for hostname in hostnames {
        let ip = dns_lookup::lookup_host(hostname);
        match ip {
            Ok(ip) => ips.insert(hostname.to_string(), Ok(ip[0].to_string())),
            Err(e) => ips.insert(hostname.to_string(), Err(e.to_string())),
        };
    }
    ips
}