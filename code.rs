use crossbeam::queue::ArrayQueue;
use dashmap::DashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// --- Constants & Types ---
const PAGE_SIZE: usize = 4096;
const TIER0_CAPACITY: usize = 1024;

type PageId = u64;

#[derive(Clone, Debug)]
pub struct RawPage {
    pub id: PageId,
    pub data: [u8; PAGE_SIZE],
    pub min_timestamp: u64,
}

pub struct TemporalPage {
    pub raw: RawPage,
    pub access_count: u32,
    pub last_access: u64,
}

impl TemporalPage {
    fn new(raw: RawPage) -> Self {
        Self {
            raw,
            access_count: 1,
            last_access: current_time(),
        }
    }

    pub fn calculate_score(&self) -> f64 {
        let now = current_time();
        let delta_t = (now - self.raw.min_timestamp).max(1);
        (self.access_count as f64) / (delta_t as f64)
    }
}

pub struct TSBufferManager {
    tier0: ArrayQueue<RawPage>,
    tier1: DashMap<PageId, TemporalPage>,
    tier2: DashMap<PageId, Vec<u8>>,
}

impl TSBufferManager {
    pub fn new() -> Self {
        Self {
            tier0: ArrayQueue::new(TIER0_CAPACITY),
            tier1: DashMap::new(),
            tier2: DashMap::new(),
        }
    }

    pub fn ingest(&self, page: RawPage) -> Result<(), &str> {
        self.tier0.push(page).map_err(|_| "Tier 0 Buffer Full")
    }

    pub fn process_ingestion(&self) {
        while let Some(raw_page) = self.tier0.pop() {
            let id = raw_page.id;
            self.tier1.insert(id, TemporalPage::new(raw_page));
        }
    }

    pub fn get_page(&self, id: PageId) -> Option<RawPage> {
        if let Some(mut entry) = self.tier1.get_mut(&id) {
            entry.access_count += 1;
            entry.last_access = current_time();
            return Some(entry.raw.clone());
        }
        None
    }

    pub fn maintenance_evict(&self) {
        if self.tier1.is_empty() { return; }

        // FIX: Explicitly typed the reference to solve E0282
        let mut items: Vec<(PageId, f64)> = self.tier1
            .iter()
            .map(|entry| {
                let (id, page) = entry.pair();
                (*id, page.calculate_score())
            })
            .collect();

        items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        if let Some((id_to_evict, _)) = items.first() {
            if let Some((_, _page)) = self.tier1.remove(id_to_evict) {
                println!("Evicting Page ID: {} to Compressed Tier 2", id_to_evict);
                self.tier2.insert(*id_to_evict, vec![0; 128]); 
            }
        }
    }
}

fn current_time() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

// --- Entry Point ---
fn main() {
    let manager = TSBufferManager::new();
    let now = current_time();

    println!("🚀 Starting TS Buffer Manager Demo...");

    // Simulate ingestion
    let p1 = RawPage { id: 1, data: [0; PAGE_SIZE], min_timestamp: now - 60 };
    let p2 = RawPage { id: 2, data: [0; PAGE_SIZE], min_timestamp: now };
    
    manager.ingest(p1).unwrap();
    manager.ingest(p2).unwrap();
    
    println!("Ingested 2 pages into Tier 0.");
    
    manager.process_ingestion();
    println!("Pages moved to Tier 1 Temporal Cache.");

    // Evict the least valuable page
    manager.maintenance_evict();
    
    println!("Buffer health check complete.");
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ts_logic() {
        let manager = TSBufferManager::new();
        let now = current_time();
        let p = RawPage { id: 10, data: [0; PAGE_SIZE], min_timestamp: now };
        manager.ingest(p).unwrap();
        manager.process_ingestion();
        assert!(manager.tier1.contains_key(&10));
    }
}
