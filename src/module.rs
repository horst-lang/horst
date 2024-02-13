use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path};
use crate::gc::GcTrace;
use crate::value::Value;
use crate::vm::VM;

pub struct LoadModuleResult {
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub(crate) name: String,
    pub(crate) variables: HashMap<String, Value>,
}

impl Module {
    pub fn new(name: String) -> Module {
        Module {
            name,
            variables: HashMap::new(),
        }
    }
}

impl GcTrace for Module {
    fn size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn trace(&self, _gc: &mut crate::gc::Gc) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn is_relative_import(module: &str) -> bool {
    module.starts_with('.')
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .map(|component| match component {
            Component::Prefix(s) => s.as_os_str().to_str().unwrap(),
            Component::RootDir => "",
            Component::CurDir => ".",
            Component::ParentDir => "..",
            Component::Normal(s) => s.to_str().unwrap(),
        })
        .collect::<Vec<&str>>()
        .join("/")
}

pub fn resolve_module(_vm: &VM, importer: &str, module: &str) -> String {
    if !is_relative_import(module) {
        return module.into();
    }

    let importer_path = Path::new(importer);
    let importer_dir = importer_path.parent().unwrap();
    let resolved = importer_dir.join(module);
    normalize_path(&resolved)
}

pub fn read_module(_vm: &VM, name: &str) -> Option<LoadModuleResult> {
    let path = format!("{}.horst", name);
    if let Ok(source) = fs::read_to_string(&path) {
        Some(LoadModuleResult { source })
    } else {
        None
    }
}