// use super::super::{Address, Profile};
// use crate::network::p2p::NodeId;
// pub use jormungandr_lib::interfaces::{PreferredListConfig, TrustedPeer};
// use poldercast::layer::ViewBuilder;
// use poldercast::{layer::Layer, InterestLevel, Topic};
// use rand::seq::IteratorRandom;
// use rand_chacha::ChaChaRng;
// use std::collections::HashSet;

// pub struct PreferredListLayer {
//     /// the max number of entries to add in the list of the view
//     view_max: usize,

//     /// the buddy list
//     // FIXME: This
//     peers: HashSet<TrustedPeer>,

//     /// a pseudo random number generator, this will help with
//     /// testing and reproducing issues.
//     ///
//     /// Do not let a user seed the value, while having a secure
//     /// RNG is not necessary it is functionally important to allow
//     /// for randomness to make its course.
//     prng: rand_chacha::ChaChaRng,
// }

// impl PreferredListLayer {
//     pub fn new(config: PreferredListConfig, prng: ChaChaRng) -> Self {
//         let addresses: Vec<Address> = config
//             .peers
//             .iter()
//             .filter_map(|p| jormungandr_lib::multiaddr::to_tcp_socket_addr(&p.address))
//             .collect();
//         Self {
//             view_max: config.view_max.into(),
//             peers: addresses.into_iter().collect(),
//             prng,
//         }
//     }
// }

// impl Layer for PreferredListLayer {
//     fn name(&self) -> &'static str {
//         "custom::preferred_list"
//     }

//     fn view(&mut self, builder: &mut ViewBuilder) {
//         self.peers
//             .iter()
//             .choose_multiple(&mut self.prng, self.view_max)
//             .into_iter()
//             .for_each(|address| builder.add(address.clone()));
//     }

//     fn remove(&mut self, id: &NodeId) {}
//     fn reset(&mut self) {}
//     fn subscribe(&mut self, topic: Topic) {}
//     fn unsubscribe(&mut self, topic: &Topic) {}
//     fn subscriptions(&self, output: &mut PriorityMap<InterestLevel, Topic>) {}

//     fn populate(&mut self, our_profile: &Profile, new_profile: &Profile) {}
// }
