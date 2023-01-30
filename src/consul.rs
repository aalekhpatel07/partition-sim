use std::{process::{Command, Stdio}, net::IpAddr};


/// The main Consul api that the supervisor uses to query a list of peer
/// ip addresses and ports they registered their services with.
/// 
pub fn query_consul_for_peers(
    dns_addr: &str,
    dns_port: u16,
    service_name: &str,
) -> crate::Result<Vec<(IpAddr, u16)>> {

    let child = Command::new("dig")
        .arg("+short")
        .arg(format!("@{}", dns_addr))
        .arg("-p")
        .arg(format!("{}", dns_port))
        .arg("-t")
        .arg("SRV")
        .arg(format!("{}.service.consul", service_name))
        .stdout(Stdio::piped())
        .spawn()?;
    
    let child = Command::new("awk")
        .arg("{print $4,$3;}")
        .stdin(child.stdout.expect("No stdout of dig command?"))
        .stdout(Stdio::piped())
        .output()?;

    let output = String::from_utf8_lossy(&child.stdout);

    let dns_names_and_ports: Vec<_> =
    output
    .lines()
    .map(|line| {
        let mut parts = line.split_whitespace();
        let dns_name = parts.next().unwrap();
        let port = parts.next().unwrap();
        (dns_name, port)
    })
    .collect();

    let dns_names = 
    dns_names_and_ports
    .iter()
    .map(|&(dns_name, _)| {
        dns_name
    })
    .collect::<Vec<_>>();

    let output = 
        Command::new("dig")
        .arg("+short")
        .arg(format!("@{}", dns_addr))
        .arg("-p")
        .arg(format!("{}", dns_port))
        .args(&dns_names)
        .output()?;

    let output = String::from_utf8_lossy(&output.stdout);

    Ok(
        output
        .lines()
        .map(|line| {
            line.to_string()
        })
        .zip(
            dns_names_and_ports
            .into_iter()
            .map(|(_, port)| port)
        )
        .map(|(ip_addr_raw, port)| {
            let ip_addr = ip_addr_raw.parse::<IpAddr>().unwrap();
            (ip_addr, port.parse::<u16>().unwrap())
        })
        .collect()
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::net::IpAddr;

    use super::query_consul_for_peers;

    /// This test must be run after `docker-compose run -d` has been run in the root of the project.
    #[test]
    pub fn stuff() {
        let res = query_consul_for_peers(
            "127.0.0.1", 
            8600, 
            "test-node-base"
        ).unwrap();
        println!("{:#?}", res);
        let observed : HashSet<(IpAddr, u16)> = std::collections::HashSet::from_iter(res.into_iter());
        let expected = std::collections::HashSet::from_iter(
            vec![
                ("192.168.192.4".parse().unwrap(), 9001), 
                ("192.168.192.6".parse().unwrap(), 9001), 
                ("192.168.192.7".parse().unwrap(), 9001), 
                ("192.168.192.8".parse().unwrap(), 9001), 
                ("192.168.192.5".parse().unwrap(), 9001)
            ]
        );
        assert_eq!(observed, expected);
    }
}