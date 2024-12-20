use std::collections::{HashMap, HashSet};

use crate::as_graphs::caida_as_graph::as_graph::{AutonomousSystem};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Relationships {
    PROVIDERS = 1,
    PEERS = 2,
    CUSTOMERS = 3,
    ORIGIN = 4,
    UNKNOWN = 5,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Announcement {
    pub prefix: String,
    pub as_path: Vec<u32>,
    pub next_hop_asn: Option<u32>,
    pub seed_asn: Option<u32>,
    pub recv_relationship: Relationships,
    pub timestamp: u64,
    pub withdraw: bool,
    pub bgpsec_next_asn: Option<u32>,
    pub bgpsec_as_path: Vec<u32>,
    pub only_to_customers: Option<u32>,
    pub rovpp_blackhole: bool,
}

impl Announcement {
    pub fn new(prefix: String, as_path: Vec<u32>, recv_relationship: Relationships) -> Self {
        let next_hop_asn = if as_path.len() == 1 {
            Some(as_path[0])
        } else {
            None
        };

        let seed_asn = if as_path.len() == 1 {
            Some(as_path[0])
        } else {
            None
        };

        Self {
            prefix,
            as_path,
            next_hop_asn,
            seed_asn,
            recv_relationship,
            timestamp: 0,
            withdraw: false,
            bgpsec_next_asn: None,
            bgpsec_as_path: Vec::new(),
            only_to_customers: None,
            rovpp_blackhole: false,
        }
    }

    pub fn prefix_path_attributes_eq(&self, other: &Announcement) -> bool {
        self.prefix == other.prefix && self.as_path == other.as_path
    }

    pub fn bgpsec_valid(&self, asn: u32) -> bool {
        self.bgpsec_next_asn == Some(asn) && self.bgpsec_as_path == self.as_path
    }

    pub fn origin(&self) -> u32 {
        *self.as_path.last().expect("AS path should not be empty")
    }
}

#[derive(Debug, Clone)]
pub struct LocalRIB {
    pub data: HashMap<String, Announcement>,
}

impl LocalRIB {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn add_ann(&mut self, ann: Announcement) {
        self.data.insert(ann.prefix.clone(), ann);
    }
}

#[derive(Debug, Clone)]
pub struct RecvQueue {
    pub data: HashMap<String, Vec<Announcement>>,
}

impl RecvQueue {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn add_ann(&mut self, ann: Announcement) {
        self.data.entry(ann.prefix.clone()).or_default().push(ann);
    }

    pub fn get_ann_list(&self, prefix: &str) -> Vec<Announcement> {
        self.data.get(prefix).cloned().unwrap_or_default()
    }
}


#[derive(Debug, Clone)]
pub struct BGP {
    pub local_rib: LocalRIB,
    pub recv_q: RecvQueue,
    pub autonomous_system: *const AutonomousSystem,
}

impl BGP {
    pub fn new(autonomous_system: *const AutonomousSystem) -> Self {
        Self {
            local_rib: LocalRIB::new(),
            recv_q: RecvQueue::new(),
            autonomous_system,
        }
    }

    pub fn seed_ann(&mut self, ann: Announcement) {
        assert!(self.local_rib.data.get(&ann.prefix).is_none(), "Seeding conflict");
        self.local_rib.add_ann(ann);
    }

    pub fn receive_ann(&mut self, ann: Announcement) {
        self.recv_q.add_ann(ann);
    }

    pub fn process_incoming_anns(&mut self, from_rel: Relationships, reset_q: bool) {
        for (prefix, ann_list) in &self.recv_q.data {
            let mut current_ann = self.local_rib.data.get(prefix);
            let original_ann = current_ann;

            if let Some(current) = current_ann {
                if current.seed_asn.is_some() {
                    continue;
                }
            }

            for new_ann in ann_list {
                if self.valid_ann(new_ann, from_rel.clone()) {
                    let processed_ann = self.copy_and_process(new_ann, from_rel.clone());

                    if current_ann.is_none() || self.is_better_ann(current_ann.unwrap(), &processed_ann) {
                        self.local_rib.add_ann(processed_ann);
                        current_ann = self.local_rib.data.get(prefix);
                    }
                }
            }

            if original_ann != current_ann {
                assert!(current_ann.is_some(), "No announcement found in local_rib after processing");
                if let Some(current) = current_ann {
                    let autonomous_system = unsafe { &*self.autonomous_system };
                    assert!(current.seed_asn.is_none() || current.seed_asn == Some(autonomous_system.asn), "Seed ASN is incorrect");
                }
            }
        }

        if reset_q {
            self.reset_recv_q();
        }
    }

    fn is_better_ann(&self, current: &Announcement, new: &Announcement) -> bool {
        if current.recv_relationship != new.recv_relationship {
            return current.recv_relationship < new.recv_relationship;
        }

        if current.as_path.len() != new.as_path.len() {
            return current.as_path.len() > new.as_path.len();
        }

        current.next_hop_asn > new.next_hop_asn
    }

    pub fn copy_and_process(&self, ann: &Announcement, recv_relationship: Relationships) -> Announcement {
        let autonomous_system = unsafe { &*self.autonomous_system };
        let mut new_as_path = vec![autonomous_system.asn];
        new_as_path.extend(&ann.as_path);

        Announcement {
            prefix: ann.prefix.clone(),
            as_path: new_as_path,
            next_hop_asn: Some(autonomous_system.asn),
            seed_asn: None,
            recv_relationship,
            timestamp: ann.timestamp,
            withdraw: ann.withdraw,
            bgpsec_next_asn: ann.bgpsec_next_asn,
            bgpsec_as_path: ann.bgpsec_as_path.clone(),
            only_to_customers: ann.only_to_customers,
            rovpp_blackhole: ann.rovpp_blackhole,
        }
    }

    pub fn propagate_to_providers(&self) {
        let neighbors = unsafe { &(*self.autonomous_system).providers };
        let send_rels: HashSet<Relationships> = [
            Relationships::ORIGIN,
            Relationships::CUSTOMERS,
        ]
        .iter()
        .cloned()
        .collect();

        self.propagate(neighbors, send_rels, Relationships::PROVIDERS);
    }

    pub fn propagate_to_customers(&self) {
        let neighbors = unsafe { &(*self.autonomous_system).customers };
        let send_rels: HashSet<Relationships> = [
            Relationships::ORIGIN,
            Relationships::CUSTOMERS,
            Relationships::PEERS,
            Relationships::PROVIDERS,
        ]
        .iter()
        .cloned()
        .collect();

        self.propagate(neighbors, send_rels, Relationships::CUSTOMERS);
    }

    pub fn propagate_to_peers(&self) {
        let neighbors = unsafe { &(*self.autonomous_system).peers };
        let send_rels: HashSet<Relationships> = [
            Relationships::ORIGIN,
            Relationships::CUSTOMERS,
        ]
        .iter()
        .cloned()
        .collect();

        self.propagate(neighbors, send_rels, Relationships::PEERS);
    }

    fn propagate(&self, neighbors: &Vec<*mut AutonomousSystem>, send_rels: HashSet<Relationships>, rel_type: Relationships) {
        for (_prefix, unprocessed_ann) in &self.local_rib.data {
            if send_rels.contains(&unprocessed_ann.recv_relationship) {
                let ann = Announcement {
                    prefix: unprocessed_ann.prefix.clone(),
                    as_path: unprocessed_ann.as_path.clone(),
                    next_hop_asn: Some(unsafe { &*self.autonomous_system }.asn),
                    seed_asn: unprocessed_ann.seed_asn,
                    recv_relationship: rel_type.clone(),
                    timestamp: unprocessed_ann.timestamp,
                    withdraw: unprocessed_ann.withdraw,
                    bgpsec_next_asn: unprocessed_ann.bgpsec_next_asn,
                    bgpsec_as_path: unprocessed_ann.bgpsec_as_path.clone(),
                    only_to_customers: unprocessed_ann.only_to_customers,
                    rovpp_blackhole: unprocessed_ann.rovpp_blackhole,
                };

                for &neighbor in neighbors {
                    self.process_outgoing_ann(neighbor, &ann, rel_type.clone(), &send_rels);
                }
            }
        }
    }

    fn valid_ann(&self, ann: &Announcement, recv_relationship: Relationships) -> bool {
        let autonomous_system = unsafe { &*self.autonomous_system };
        !ann.as_path.contains(&autonomous_system.asn) && !ann.as_path.contains(&0)
    }

    fn process_outgoing_ann(&self, neighbor: *mut AutonomousSystem, ann: &Announcement, _rel_type: Relationships, _send_rels: &HashSet<Relationships>) {
        println!("Sending {:?} to neighbor ", ann);
    }

    fn reset_recv_q(&mut self) {
        self.recv_q = RecvQueue::new();
    }
}
