use std::collections::{HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use crate::as_graphs::base::as_graph_collector::{BaseASGraphCollector, ASGraphCollector};
use crate::as_graphs::CAIDAASGraphCollector;

#[derive(Debug, Clone)]
pub struct ASGraphInfo {
    pub customer_provider_links: HashSet<CustomerProviderLink>,
    pub peer_links: HashSet<PeerLink>,
    pub ixp_asns: HashSet<u32>,
    pub input_clique_asns: HashSet<u32>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CustomerProviderLink {
    pub customer_asn: u32,
    pub provider_asn: u32,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PeerLink {
    pub peer1_asn: u32,
    pub peer2_asn: u32,
}

pub struct CAIDAASGraphConstructor {
    collector: CAIDAASGraphCollector,
}

impl CAIDAASGraphConstructor {
    pub fn new(collector: CAIDAASGraphCollector) -> Self {
        Self { collector }
    }

    pub fn run(&self) -> Result<ASGraphInfo, Box<dyn std::error::Error>> {
        let dl_path = self.collector.run();
        let as_graph_info = self.get_as_graph_info(&dl_path)?;
        Ok(as_graph_info)
    }

    fn get_as_graph_info(&self, dl_path: &PathBuf) -> Result<ASGraphInfo, Box<dyn std::error::Error>> {
        let file = File::open(dl_path)?;
        let reader = BufReader::new(file);

        let mut input_clique_asns = HashSet::new();
        let mut ixp_asns = HashSet::new();
        let mut cp_links = HashSet::new();
        let mut peer_links = HashSet::new();

        for line in reader.lines() {
            let line = line?;
            if line.starts_with("# input clique") {
                self.extract_input_clique_asns(&line, &mut input_clique_asns);
            } else if line.starts_with("# IXP ASes") {
                self.extract_ixp_asns(&line, &mut ixp_asns);
            } else if !line.starts_with("#") {
                if line.contains("-1") {
                    self.extract_provider_customers(&line, &mut cp_links);
                } else {
                    self.extract_peers(&line, &mut peer_links);
                }
            }
        }

        Ok(ASGraphInfo {
            customer_provider_links: cp_links,
            peer_links,
            ixp_asns,
            input_clique_asns,
        })
    }

    fn extract_input_clique_asns(&self, line: &str, input_clique_asns: &mut HashSet<u32>) {
        if let Some(asns) = line.split(':').last() {
            for asn in asns.trim().split_whitespace() {
                if let Ok(asn) = asn.parse::<u32>() {
                    input_clique_asns.insert(asn);
                }
            }
        }
    }

    fn extract_ixp_asns(&self, line: &str, ixp_asns: &mut HashSet<u32>) {
        if let Some(asns) = line.split(':').last() {
            for asn in asns.trim().split_whitespace() {
                if let Ok(asn) = asn.parse::<u32>() {
                    ixp_asns.insert(asn);
                }
            }
        }
    }

    fn extract_provider_customers(&self, line: &str, cp_links: &mut HashSet<CustomerProviderLink>) {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            if let (Ok(provider_asn), Ok(customer_asn)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                cp_links.insert(CustomerProviderLink {
                    provider_asn,
                    customer_asn,
                });
            }
        }
    }

    fn extract_peers(&self, line: &str, peer_links: &mut HashSet<PeerLink>) {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            if let (Ok(peer1_asn), Ok(peer2_asn)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                peer_links.insert(PeerLink {
                    peer1_asn,
                    peer2_asn,
                });
            }
        }
    }

}
