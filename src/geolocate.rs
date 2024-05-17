use hyper::{body, Request};
use std::{collections::HashMap, io::Read};

const LOCATION_DATA: &'static [u8] = include_bytes!("data/locations_256.json.gz");

pub struct IpLocator {
    locations: Vec<(u32, f64, f64)>,
    num_proxies: usize,
}

impl IpLocator {
    pub fn new(num_proxies: usize) -> Self {
        let mut reader = flate2::read::GzDecoder::new(LOCATION_DATA);
        let mut buf = Vec::new();
        reader
            .read_to_end(&mut buf)
            .expect("decode static location data");
        let parsed: HashMap<String, (f64, f64)> =
            serde_json::from_slice(&buf).expect("parse static location data");
        let mut locations = Vec::new();
        for (k, v) in parsed.into_iter() {
            let parts: Vec<u32> = k.split(".").map(|x| x.parse().expect("parse IP")).collect();
            locations.push((
                parts[0] * 0x1000000 + parts[1] * 0x10000 + parts[2] * 0x100 + parts[3],
                v.0,
                v.1,
            ));
        }
        println!("loaded IP location DB with {} entries", locations.len());
        IpLocator {
            locations,
            num_proxies: num_proxies,
        }
    }

    pub fn lookup_for_request(
        &self,
        req: &Request<body::Incoming>,
        addr: &str,
    ) -> Option<(f64, f64)> {
        if self.num_proxies > 0 {
            if let Some(forwarded) = req.headers().get("x-forwarded-for") {
                let addrs: Vec<&[u8]> = forwarded.as_bytes().split(|x| *x == b',').collect();
                if addrs.len() < self.num_proxies {
                    return None;
                }
                if let Ok(addr) =
                    String::from_utf8(addrs[addrs.len() - self.num_proxies].to_owned())
                {
                    return self.lookup(addr.trim());
                }
            }
        }
        self.lookup(addr)
    }

    pub fn lookup(&self, ip: &str) -> Option<(f64, f64)> {
        let parts: Vec<&str> = ip.split(".").collect();
        if parts.len() != 4 {
            return None;
        }
        let parsed: Result<Vec<u32>, _> = parts.iter().map(|x| x.parse()).collect();
        if let Ok(components) = parsed {
            if !components.iter().all(|x| *x < 256) {
                // Avoid overflow
                return None;
            }
            let ip_num = components[0] * 0x1000000
                + components[1] * 0x10000
                + components[2] * 0x100
                + components[3];
            let mut min_dist: u32 = 0xffffffff;
            let mut result = (0.0, 0.0);
            for (cur_ip, lat, lon) in &self.locations {
                let dist = if *cur_ip > ip_num {
                    cur_ip - ip_num
                } else {
                    ip_num - cur_ip
                };
                if dist <= min_dist {
                    min_dist = dist;
                    result = (*lat, *lon);
                }
            }
            Some(result)
        } else {
            None
        }
    }
}
