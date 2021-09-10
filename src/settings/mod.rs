use crate::auth;
use crate::console;
use crate::paths;
use std::any::Any;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::marker::PhantomData;

pub mod vars;

pub struct CVar<T: Sized + Any + 'static> {
    pub name: &'static str,
    pub ty: PhantomData<T>,
    pub description: &'static str,
    pub mutable: bool,
    pub serializable: bool,
    pub default: &'static dyn Fn() -> T,
}

impl Var for CVar<i64> {
    fn serialize(&self, val: &Box<dyn Any>) -> String {
        val.downcast_ref::<i64>().unwrap().to_string()
    }

    fn deserialize(&self, input: &str) -> Box<dyn Any> {
        Box::new(input.parse::<i64>().unwrap())
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn can_serialize(&self) -> bool {
        self.serializable
    }
}

impl Var for CVar<bool> {
    fn serialize(&self, val: &Box<dyn Any>) -> String {
        val.downcast_ref::<bool>().unwrap().to_string()
    }

    fn deserialize(&self, input: &str) -> Box<dyn Any> {
        Box::new(input.parse::<bool>().unwrap())
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn can_serialize(&self) -> bool {
        self.serializable
    }
}

impl Var for CVar<String> {
    fn serialize(&self, val: &Box<dyn Any>) -> String {
        format!("\"{}\"", val.downcast_ref::<String>().unwrap())
    }

    fn deserialize(&self, input: &str) -> Box<dyn Any> {
        Box::new((&input[1..input.len() - 1]).to_owned())
    }

    fn description(&self) -> &'static str {
        self.description
    }
    fn can_serialize(&self) -> bool {
        self.serializable
    }
}

pub trait Var {
    fn serialize(&self, val: &Box<dyn Any>) -> String;
    fn deserialize(&self, input: &str) -> Box<dyn Any>;
    fn description(&self) -> &'static str;
    fn can_serialize(&self) -> bool;
}

#[derive(Default)]
pub struct Vars {
    names: HashMap<String, &'static str>,
    vars: HashMap<&'static str, Box<dyn Var>>,
    var_values: HashMap<&'static str, RefCell<Box<dyn Any>>>,
}

impl Vars {
    pub fn new() -> Vars {
        let mut vars: Vars = Default::default();
        vars::register_vars(&mut vars);
        console::register_vars(&mut vars);
        auth::register_vars(&mut vars);
        vars.load_config();
        vars
    }

    pub fn register<T: Sized + Any>(&mut self, var: CVar<T>)
    where
        CVar<T>: Var,
    {
        if self.vars.contains_key(var.name) {
            panic!("Key registered twice {}", var.name);
        }
        self.names.insert(var.name.to_owned(), var.name);
        self.var_values
            .insert(var.name, RefCell::new(Box::new((var.default)())));
        self.vars.insert(var.name, Box::new(var));
    }

    pub fn get<T: Sized + Any>(&self, var: CVar<T>) -> Ref<T>
    where
        CVar<T>: Var,
    {
        // Should never fail
        let var = self.var_values.get(var.name).unwrap().borrow();
        Ref::map(var, |v| v.downcast_ref::<T>().unwrap())
    }

    pub fn set<T: Sized + Any>(&self, var: CVar<T>, val: T)
    where
        CVar<T>: Var,
    {
        *self.var_values.get(var.name).unwrap().borrow_mut() = Box::new(val);
        self.save_config();
    }

    fn load_config(&mut self) {
        if let Ok(file) = fs::File::open(paths::get_config_dir().join("conf.cfg")) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line.unwrap();
                if line.starts_with('#') || line.is_empty() {
                    continue;
                }
                let parts = line
                    .splitn(2, ' ')
                    .map(|v| v.to_owned())
                    .collect::<Vec<String>>();
                let (name, arg) = (&parts[0], &parts[1]);
                if let Some(var_name) = self.names.get(name) {
                    let var = self.vars.get(var_name).unwrap();
                    let val = var.deserialize(arg);
                    if var.can_serialize() {
                        self.var_values.insert(var_name, RefCell::new(val));
                    }
                }
            }
        }
    }

    pub fn save_config(&self) {
        let mut file =
            BufWriter::new(fs::File::create(paths::get_config_dir().join("conf.cfg")).unwrap());
        for (name, var) in &self.vars {
            if !var.can_serialize() {
                continue;
            }
            for line in var.description().lines() {
                writeln!(file, "# {}", line).unwrap();
            }
            write!(
                file,
                "{} {}\n\n",
                name,
                var.serialize(&self.var_values.get(name).unwrap().borrow())
            )
            .unwrap();
        }
    }
}
