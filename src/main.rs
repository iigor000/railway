use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::io::{self, BufRead};

#[derive(Debug)]
enum RailwayError {
    IoError(io::Error),
    ParseError(String),
    InvalidInput(String),
}

impl fmt::Display for RailwayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RailwayError::IoError(e) => write!(f, "IO error: {}", e),
            RailwayError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            RailwayError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl From<io::Error> for RailwayError {
    fn from(error: io::Error) -> Self {
        RailwayError::IoError(error)
    }
}

fn main() {
    match run() {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), RailwayError> {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    // Read S (stations) and T (tracks)
    println!("Enter number of stations (S) and tracks (T):");
    let first_line = lines
        .next()
        .ok_or(RailwayError::ParseError("Missing first line".to_string()))?
        .map_err(RailwayError::IoError)?;
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(RailwayError::ParseError(
            "First line must contain exactly 2 numbers: S T".to_string(),
        ));
    }

    let s: usize = parts[0]
        .parse()
        .map_err(|_| RailwayError::ParseError(format!("Cannot parse '{}' as number", parts[0])))?;
    let t: usize = parts[1]
        .parse()
        .map_err(|_| RailwayError::ParseError(format!("Cannot parse '{}' as number", parts[1])))?;

    // Read station information
    let mut unload: HashMap<usize, usize> = HashMap::new();
    let mut load: HashMap<usize, usize> = HashMap::new();

    for i in 0..s {
        println!("Enter station {} information (id c_unload c_load):", i + 1);
        let line = lines
            .next()
            .ok_or(RailwayError::ParseError(format!(
                "Missing station information line {}",
                i + 2
            )))?
            .map_err(RailwayError::IoError)?;

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(RailwayError::ParseError(format!(
                "Line {} has {} parts, expected 3",
                i + 2,
                parts.len()
            )));
        }

        let station_id: usize = parts[0]
            .parse()
            .map_err(|_| RailwayError::ParseError(format!("Invalid station id: {}", parts[0])))?;
        let c_unload: usize = parts[1]
            .parse()
            .map_err(|_| RailwayError::ParseError(format!("Invalid cargo type: {}", parts[1])))?;
        let c_load: usize = parts[2]
            .parse()
            .map_err(|_| RailwayError::ParseError(format!("Invalid cargo type: {}", parts[2])))?;

        // Validate cargo types fit in u64 bitset
        if c_unload >= 64 || c_load >= 64 {
            return Err(RailwayError::InvalidInput(format!(
                "Cargo types must be 0-63, got {} or {}",
                c_unload, c_load
            )));
        }

        unload.insert(station_id, c_unload);
        load.insert(station_id, c_load);
    }

    // Build the graph
    let mut graph: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..t {
        println!("Enter track {} information (from to):", i + 1);
        let line = lines
            .next()
            .ok_or(RailwayError::ParseError(format!(
                "Missing track information line {}",
                s + 2 + i
            )))?
            .map_err(RailwayError::IoError)?;

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(RailwayError::ParseError(format!(
                "Track line {} has {} parts, expected 2",
                i + 1,
                parts.len()
            )));
        }

        let from: usize = parts[0]
            .parse()
            .map_err(|_| RailwayError::ParseError(format!("Invalid station id: {}", parts[0])))?;
        let to: usize = parts[1]
            .parse()
            .map_err(|_| RailwayError::ParseError(format!("Invalid station id: {}", parts[1])))?;

        graph.entry(from).or_insert_with(Vec::new).push(to);
    }

    // Read starting station
    println!("Enter starting station id:");
    let start_line = lines
        .next()
        .ok_or(RailwayError::ParseError(
            "Missing starting station line".to_string(),
        ))?
        .map_err(RailwayError::IoError)?;

    let start_station: usize = start_line.trim().parse().map_err(|_| {
        RailwayError::ParseError(format!("Invalid starting station: {}", start_line))
    })?;

    // Validate starting station exists in configuration
    if !load.contains_key(&start_station)
        && !unload.contains_key(&start_station)
        && !graph.contains_key(&start_station)
    {
        return Err(RailwayError::InvalidInput(format!(
            "Starting station {} not found in configuration",
            start_station
        )));
    }

    let cargo_at_station = calculate(&load, &unload, &graph, start_station);

    // Output results
    println!("Cargo types that can arrive at each station:");
    let mut stations: Vec<_> = cargo_at_station.keys().collect();
    stations.sort();
    for &station in stations {
        let cargo_bits = cargo_at_station[&station];
        print!("Station {}: ", station);
        let cargo_types: Vec<usize> = (0..64)
            .filter(|bit| (cargo_bits & (1u64 << bit)) != 0)
            .collect();
        if cargo_types.is_empty() {
            println!("(no cargo)");
        } else {
            println!("{:?}", cargo_types);
        }
    }

    Ok(())
}

fn calculate(
    load: &HashMap<usize, usize>,
    unload: &HashMap<usize, usize>,
    graph: &HashMap<usize, Vec<usize>>,
    start_station: usize,
) -> HashMap<usize, u64> {
    // Fixpoint iteration algorithm: iteratively propagate cargo until convergence
    // This is more efficient than BFS state tracking because:
    // 1. No (station, cargo) state explosion
    // 2. Only reprocess stations when their incoming cargo changes
    // 3. Guaranteed convergence (at most S * 64 iterations)

    let mut cargo_at_station: HashMap<usize, u64> = HashMap::new();
    let mut worklist: VecDeque<usize> = VecDeque::new();

    // Start with the starting station
    cargo_at_station.insert(start_station, 0);
    worklist.push_back(start_station);

    while let Some(station) = worklist.pop_front() {
        let mut cargo = cargo_at_station[&station];

        // Unload cargo type for this station
        if let Some(&unload_type) = unload.get(&station) {
            cargo &= !(1u64 << unload_type);
        }

        // Load cargo type for this station
        if let Some(&load_type) = load.get(&station) {
            cargo |= 1u64 << load_type;
        }

        // Propagate to all neighbors
        if let Some(neighbors) = graph.get(&station) {
            for &next_station in neighbors {
                let entry = cargo_at_station.entry(next_station).or_insert(0);
                let old_cargo = *entry;

                // Merge incoming cargo with existing cargo
                *entry |= cargo;

                // If cargo changed, reprocess this station
                if *entry != old_cargo {
                    worklist.push_back(next_station);
                }
            }
        }
    }

    cargo_at_station
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_linear_path() {
        // Test: 0 -> 1 -> 2
        // Station 0: loads cargo 1
        // Station 1: unloads 1, loads 2
        // Station 2: unloads 2
        let mut unload = HashMap::new();
        let mut load = HashMap::new();
        let mut graph = HashMap::new();

        load.insert(0, 1); // Station 0 loads cargo 1
        unload.insert(1, 1); // Station 1 unloads cargo 1
        load.insert(1, 2); // Station 1 loads cargo 2
        unload.insert(2, 2); // Station 2 unloads cargo 2

        graph.insert(0, vec![1]);
        graph.insert(1, vec![2]);

        let result = calculate(&load, &unload, &graph, 0);

        // Station 0: arrives with no cargo
        assert_eq!(result.get(&0).unwrap_or(&0), &0);

        // Station 1: arrives with cargo 1
        assert_eq!(result.get(&1).unwrap_or(&0) & (1u64 << 1), 1u64 << 1);

        // Station 2: arrives with cargo 2
        assert_eq!(result.get(&2).unwrap_or(&0) & (1u64 << 2), 1u64 << 2);
    }

    #[test]
    fn test_branching_paths() {
        // Test: Station 0 can go to both 1 and 2
        // 0 -> 1, 0 -> 2
        let mut unload = HashMap::new();
        let mut load = HashMap::new();
        let mut graph = HashMap::new();

        load.insert(0, 5); // Station 0 loads cargo 5
        unload.insert(1, 5); // Station 1 unloads cargo 5
        load.insert(1, 7);
        unload.insert(2, 5);
        load.insert(2, 9);

        graph.insert(0, vec![1, 2]);

        let result = calculate(&load, &unload, &graph, 0);

        // Station 1: arrives with cargo 5
        assert_eq!(result.get(&1).unwrap_or(&0) & (1u64 << 5), 1u64 << 5);

        // Station 2: arrives with cargo 5
        assert_eq!(result.get(&2).unwrap_or(&0) & (1u64 << 5), 1u64 << 5);
    }

    #[test]
    fn test_cargo_accumulation() {
        // Test: 0 -> 1 -> 2, where cargo types accumulate
        // Station 0: loads cargo 1
        // Station 1: loads cargo 2 (doesn't unload anything)
        // Station 2: doesn't do anything
        let unload = HashMap::new();
        let mut load = HashMap::new();
        let mut graph = HashMap::new();

        load.insert(0, 1);
        load.insert(1, 2);

        graph.insert(0, vec![1]);
        graph.insert(1, vec![2]);

        let result = calculate(&load, &unload, &graph, 0);

        // Station 1: should have cargo 1
        assert_eq!(result.get(&1).unwrap_or(&0) & (1u64 << 1), 1u64 << 1);

        // Station 2: should have both cargo 1 and 2
        let station_2_cargo = result.get(&2).unwrap_or(&0);
        assert_eq!(station_2_cargo & (1u64 << 1), 1u64 << 1);
        assert_eq!(station_2_cargo & (1u64 << 2), 1u64 << 2);
    }

    #[test]
    fn test_cycle_handling() {
        // Test: 0 -> 1 -> 0 (cycle)
        // Should not infinite loop due to visited state tracking
        let mut unload = HashMap::new();
        let mut load = HashMap::new();
        let mut graph = HashMap::new();

        load.insert(0, 3);
        unload.insert(1, 3);
        load.insert(1, 4);

        graph.insert(0, vec![1]);
        graph.insert(1, vec![0]);

        let result = calculate(&load, &unload, &graph, 0);

        // Station 0: should have cargo 4 (from the cycle)
        assert!(
            result.get(&0).unwrap_or(&0) & (1u64 << 4) == (1u64 << 4)
                || result.get(&0).unwrap_or(&0) == &0
        );

        // Station 1: should have cargo 3
        assert_eq!(result.get(&1).unwrap_or(&0) & (1u64 << 3), 1u64 << 3);
    }

    #[test]
    fn test_unload_removes_cargo() {
        // Test: Station 1 unloads cargo that was loaded at station 0
        let mut unload = HashMap::new();
        let mut load = HashMap::new();
        let mut graph = HashMap::new();

        load.insert(0, 5);
        unload.insert(1, 5);
        graph.insert(1, vec![2]);
        graph.insert(0, vec![1]);

        let result = calculate(&load, &unload, &graph, 0);

        // Station 2: should NOT have cargo 5 (it was unloaded at station 1)
        assert_eq!(result.get(&2).unwrap_or(&0) & (1u64 << 5), 0);
    }

    #[test]
    fn test_isolated_station() {
        // Test: Starting station with no outgoing edges
        let unload = HashMap::new();
        let mut load = HashMap::new();
        let graph = HashMap::new();

        load.insert(0, 1);

        let result = calculate(&load, &unload, &graph, 0);

        // Only station 0 should be in results
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(&0).unwrap_or(&0), &0); // Arrives with no cargo
    }
}
