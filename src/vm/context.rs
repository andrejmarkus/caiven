use crate::input::Input;
use crate::rendering::screen::ScreenLayer;
use crate::vm::Vm;

pub struct ExecutionContext<'a> {
    pub vm: &'a mut Vm,
    pub input: &'a Input,
    pub world: &'a mut ScreenLayer,
    pub ui: &'a mut ScreenLayer,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(
        vm: &'a mut Vm,
        input: &'a Input,
        world: &'a mut ScreenLayer,
        ui: &'a mut ScreenLayer,
    ) -> Self {
        Self {
            vm,
            input,
            world,
            ui,
        }
    }
}
