use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ItemInfo {
    Instruction {
        name: String,
        opcode: u8,
        size: usize,
    },
    Directive {
        name: String,
        size: usize,
    },
}

#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub labels: Vec<String>,
    pub item: Option<ItemInfo>,
}

pub struct SourceMap {
    map: HashMap<usize, AddressInfo>,
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert_item(&mut self, address: usize, item: ItemInfo) {
        let entry = self.map.entry(address).or_insert(AddressInfo {
            labels: Vec::new(),
            item: None,
        });
        entry.item = Some(item);
    }

    pub fn insert_label(&mut self, address: usize, label: String) {
        let entry = self.map.entry(address).or_insert(AddressInfo {
            labels: Vec::new(),
            item: None,
        });
        entry.labels.push(label);
    }

    pub fn get(&self, address: usize) -> Option<&AddressInfo> {
        self.map.get(&address)
    }

    pub fn sorted_addresses(&self) -> Vec<usize> {
        let mut addrs: Vec<usize> = self.map.keys().cloned().collect();
        addrs.sort_unstable();
        addrs
    }
}
