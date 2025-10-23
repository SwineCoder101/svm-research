#[derive(Clone, Debug)]
struct Node {
    pubkey: [u8; 32],
    stake: u64,
}

struct TurbineTree {
    fanout: usize,
    nodes: Vec<Node>,
}

impl TurbineTree {
    fn new(fanout: usize, nodes: Vec<Node>) -> Self {
        Self { fanout, nodes }
    }

    fn build_layer_matrix(&self, leader: &Node) -> Vec<Vec<Node>> {
        // 1. Sorts nodes by stake weight
        let mut sorted_nodes = self.nodes.clone();
        sorted_nodes.sort_by(|a, b| b.stake.cmp(&a.stake));

        // 2. Constructs layers with the given fanout
        let mut layers: Vec<Vec<Node>> = Vec::new();
        for i in 0..sorted_nodes.len() {
            let layer: Vec<Node> = sorted_nodes[i..].iter().take(self.fanout).cloned().collect();
            layers.push(layer);
        }

        // 3. Ensures the leader is at the root
        let leader_index = sorted_nodes.iter().position(|node| node.pubkey == leader.pubkey).unwrap();
        let leader_layer = layers.remove(leader_index);
        layers.insert(0, leader_layer);

        // 4. Optimizes for network topology
        let mut optimized_layers = Vec::new();
        for layer in layers {
            let mut layer_nodes = layer.clone();
            layer_nodes.sort_by_key(|node| node.pubkey);
            optimized_layers.push(layer_nodes);
        }

        optimized_layers
    }

    fn calculate_propagation_time(&self, layers: &[Vec<Node>]) -> u64 {
        // Calculate worst-case propagation time based on layer depth and fanout
        let mut total_time = 0;
        for (layer_index, layer) in layers.iter().enumerate() {
            // Each layer takes time proportional to its depth and number of nodes
            let layer_time = (layer_index + 1) as u64 * layer.len() as u64;
            total_time += layer_time;
        }
        total_time
    }
}

// Example usage and main function
pub fn main() {
    // Create sample nodes
    let nodes = vec![
        Node {
            pubkey: [1u8; 32],
            stake: 1000,
        },
        Node {
            pubkey: [2u8; 32],
            stake: 2000,
        },
        Node {
            pubkey: [3u8; 32],
            stake: 1500,
        },
        Node {
            pubkey: [4u8; 32],
            stake: 500,
        },
    ];

    // Create turbine tree with fanout of 2
    let turbine_tree = TurbineTree::new(2, nodes);

    // Define leader node
    let leader = Node {
        pubkey: [1u8; 32],
        stake: 1000,
    };

    // Build layer matrix
    let layers = turbine_tree.build_layer_matrix(&leader);
    
    println!("Turbine Block Propagation Layers:");
    for (i, layer) in layers.iter().enumerate() {
        println!("Layer {}: {} nodes", i, layer.len());
        for node in layer {
            println!("  Node: {:?}, Stake: {}", 
                     node.pubkey, node.stake);
        }
    }

    // Calculate total propagation time
    let total_time = turbine_tree.calculate_propagation_time(&layers);
    println!("Total propagation time: {}", total_time);
}
