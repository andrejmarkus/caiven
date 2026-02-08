use crate::assembler::Directive;

pub struct DirectiveSet {
    pub directives: Vec<Directive>,
}

impl DirectiveSet {
    pub fn new() -> Self {
        Self {
            directives: Vec::new(),
        }
    }

    pub fn register(&mut self, directive: Directive) {
        self.directives.push(directive);
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Directive> {
        self.directives.iter().find(|d| d.name == name)
    }
}
