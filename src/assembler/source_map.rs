use crate::assembler::item::AssemblyItem;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub labels: Vec<String>,
    pub item: Option<AssemblyItem>,
}

pub struct SourceMap {
    pub map: HashMap<usize, AddressInfo>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert_item(&mut self, address: usize, item: AssemblyItem) {
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
}
