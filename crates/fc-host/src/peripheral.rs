use crate::vm::memory::Memory;

pub trait Peripheral {
    fn name(&self) -> &'static str;
    fn init(&mut self, mem: &mut Memory);
    fn tick(&mut self, mem: &mut Memory, frame: u32);
}

pub struct PeripheralRegistry {
    peripherals: Vec<Box<dyn Peripheral>>,
}

impl PeripheralRegistry {
    pub fn new() -> Self {
        Self {
            peripherals: Vec::new(),
        }
    }

    pub fn register(&mut self, p: impl Peripheral + 'static) {
        self.peripherals.push(Box::new(p));
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.peripherals.iter().map(|p| p.name()).collect()
    }

    pub fn init_all(&mut self, mem: &mut Memory) {
        for p in &mut self.peripherals {
            p.init(mem);
        }
    }

    pub fn tick_all(&mut self, mem: &mut Memory, frame: u32) {
        for p in &mut self.peripherals {
            p.tick(mem, frame);
        }
    }
}
