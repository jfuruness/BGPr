use std::collections::{HashMap, HashSet};

use crate::as_graphs::caida_as_graph::{ASGraphInfo, CustomerProviderLink, PeerLink};


#[derive(Debug)]
pub struct AutonomousSystem {
    pub asn: u32,
    pub input_clique: bool,
    pub ixp: bool,
    pub peers: Vec<*mut AutonomousSystem>, // Mutable raw pointers to allow modification
    pub providers: Vec<*mut AutonomousSystem>,
    pub customers: Vec<*mut AutonomousSystem>,
    pub propagation_rank: Option<u32>,
}

impl AutonomousSystem {
    pub fn new(asn: u32, input_clique: bool, ixp: bool) -> Self {
        Self {
            asn,
            input_clique,
            ixp,
            peers: Vec::new(),
            providers: Vec::new(),
            customers: Vec::new(),
            propagation_rank: None,
        }
    }

    pub fn is_stub(&self) -> bool {
        self.neighbors().len() == 1
    }

    pub fn is_multihomed(&self) -> bool {
        self.customers.is_empty() && self.peers.len() + self.providers.len() > 1
    }

    pub fn is_transit(&self) -> bool {
        !self.customers.is_empty() && (self.customers.len() + self.peers.len() + self.providers.len() > 1)
    }

    pub fn neighbors(&self) -> Vec<u32> {
        let mut neighbors = Vec::new();
        neighbors.extend(self.customers.iter().map(|&ptr| unsafe { (*ptr).asn }));
        neighbors.extend(self.peers.iter().map(|&ptr| unsafe { (*ptr).asn }));
        neighbors.extend(self.providers.iter().map(|&ptr| unsafe { (*ptr).asn }));
        neighbors
    }

    pub fn neighbor_asns(&self) -> Vec<u32> {
        self.neighbors()
    }
}

pub struct ASGraph {
    pub as_dict: HashMap<u32, Box<AutonomousSystem>>,
    pub as_groups: HashMap<String, HashSet<u32>>,
}

impl ASGraph {
    pub fn new(as_graph_info: ASGraphInfo) -> Self {
        let mut graph = ASGraph {
            as_dict: HashMap::new(),
            as_groups: HashMap::new(),
        };

        graph.generate_graph(&as_graph_info);
        graph.add_relationships(&as_graph_info);
        graph.assign_propagation_ranks();
        graph.set_as_groups();
        graph
    }
    fn generate_graph(&mut self, as_graph_info: &ASGraphInfo) {
        fn gen_as(asn: u32, graph: &mut ASGraph) -> *mut AutonomousSystem {
            let as_obj = Box::new(AutonomousSystem::new(asn, false, false));
            let ptr = Box::into_raw(as_obj);
            graph.as_dict.insert(asn, unsafe { Box::from_raw(ptr) });
            ptr
        }

        for asn in as_graph_info.asns() {
            if !self.as_dict.contains_key(&asn) {
                gen_as(asn, self);
            }
        }

        for &asn in &as_graph_info.ixp_asns {
            let as_ptr = if let Some(as_obj) = self.as_dict.get_mut(&asn) {
                as_obj.as_mut() as *mut _
            } else {
                gen_as(asn, self)
            };
            unsafe {
                (*as_ptr).ixp = true;
            }
        }

        for &asn in &as_graph_info.input_clique_asns {
            let as_ptr = if let Some(as_obj) = self.as_dict.get_mut(&asn) {
                as_obj.as_mut() as *mut _
            } else {
                gen_as(asn, self)
            };
            unsafe {
                (*as_ptr).input_clique = true;
            }
        }
    }

    fn add_relationships(&mut self, as_graph_info: &ASGraphInfo) {
        let mut temp_relations = vec![];

        for link in &as_graph_info.customer_provider_links {
            temp_relations.push((link.customer_asn, link.provider_asn));
        }

        for (customer_asn, provider_asn) in temp_relations {
            let customer: *mut AutonomousSystem = self.as_dict.get_mut(&customer_asn).unwrap().as_mut() as *mut _;
            let provider: *mut AutonomousSystem = self.as_dict.get_mut(&provider_asn).unwrap().as_mut() as *mut _;

            unsafe {
                (*customer).providers.push(provider);
                (*provider).customers.push(customer);
            }
        }

        let mut temp_peers = vec![];
        for link in &as_graph_info.peer_links {
            temp_peers.push((link.peer1_asn, link.peer2_asn));
        }

        for (peer1_asn, peer2_asn) in temp_peers {
            let peer1: *mut AutonomousSystem = self.as_dict.get_mut(&peer1_asn).unwrap().as_mut() as *mut _;
            let peer2: *mut AutonomousSystem = self.as_dict.get_mut(&peer2_asn).unwrap().as_mut() as *mut _;

            unsafe {
                (*peer1).peers.push(peer2);
                (*peer2).peers.push(peer1);
            }
        }
    }

    fn assign_propagation_ranks(&mut self) {
        let as_nodes: Vec<*mut AutonomousSystem> = self
            .as_dict
            .values_mut()
            .map(|as_node| as_node.as_mut() as *mut _)
            .collect();

        for as_ptr in as_nodes {
            unsafe {
                self.assign_rank_recursive(as_ptr, 0);
            }
        }
    }

    unsafe fn assign_rank_recursive(&self, as_node: *mut AutonomousSystem, rank: u32) {
        if (*as_node).propagation_rank.is_none() || (*as_node).propagation_rank.unwrap() < rank {
            (*as_node).propagation_rank = Some(rank);
            for &provider in &(*as_node).providers {
                self.assign_rank_recursive(provider, rank + 1);
            }
        }
    }

    pub fn get_propagation_ranks(&self) -> Vec<Vec<u32>> {
        let mut rank_buckets: HashMap<u32, Vec<u32>> = HashMap::new();

        for as_node in self.as_dict.values() {
            if let Some(rank) = as_node.propagation_rank {
                rank_buckets.entry(rank).or_default().push(as_node.asn);
            }
        }

        let mut sorted_ranks: Vec<_> = rank_buckets.into_iter().collect();
        sorted_ranks.sort_by_key(|&(rank, _)| rank);

        sorted_ranks.into_iter().map(|(_, nodes)| nodes).collect()
    }

    fn set_as_groups(&mut self) {
        let ixp_asns: HashSet<u32> = self
            .as_dict
            .values()
            .filter(|as_obj| as_obj.ixp)
            .map(|as_obj| as_obj.asn)
            .collect();

        let stubs: HashSet<u32> = self
            .as_dict
            .values()
            .filter(|as_obj| as_obj.is_stub())
            .map(|as_obj| as_obj.asn)
            .collect();

        let multihomed: HashSet<u32> = self
            .as_dict
            .values()
            .filter(|as_obj| as_obj.is_multihomed())
            .map(|as_obj| as_obj.asn)
            .collect();

        let transit: HashSet<u32> = self
            .as_dict
            .values()
            .filter(|as_obj| as_obj.is_transit())
            .map(|as_obj| as_obj.asn)
            .collect();

        self.as_groups.insert("ixps".to_string(), ixp_asns);
        self.as_groups.insert("stubs".to_string(), stubs);
        self.as_groups.insert("multihomed".to_string(), multihomed);
        self.as_groups.insert("transit".to_string(), transit);
    }
}
