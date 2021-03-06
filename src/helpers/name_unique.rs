use std::{
    collections::{
        HashSet,
        HashMap,
    },
    fmt::Write,
};

pub fn capitalize<'input>(input: &'input str) -> impl Iterator<Item=char> + 'input {
    let (first, last) = {
        let split_index = if input.is_empty() { 0 } else { 1 };
        input.split_at(split_index)
    };
    first.chars()
        .flat_map(char::to_uppercase)
        .chain(last.chars())
}

pub fn class_style(name: &str) -> String {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s: &&str| !s.is_empty())
        .flat_map(capitalize)
        .collect()
}

pub fn generate_name(filter: impl Fn(&str) -> bool, mut name: String) -> String {
    let original_length = name.len();
    let mut counter: u128 = 0;
    while !filter(&name) {
        while name.len() > original_length {
            name.pop();
        }
        name.write_fmt(format_args!("_{}", counter))
            .expect("I/O error while writing in buffer string. WTF? OOM may be?");
        counter += 1;
    }
    name
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameUniquer {
    names: HashSet<String>,
}

impl NameUniquer {
    #[inline]
    pub fn new() -> Self {
        Self {
            names: HashSet::new(),
        }
    }
    pub fn add_name(&mut self, mut name: String) -> String {
        if self.names.contains(&name) {
            name = generate_name(
                |name| !self.names.contains(name),
                name,
            );
        }
        self.names.insert(name.clone());
        name
    }
    #[inline]
    pub fn add_class_style_name(&mut self, name: &str) -> String {
        self.add_name(class_style(name))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AliasNameUniquer<'a> {
    uniquer: &'a mut NameUniquer,
    aliases: HashMap<String, String>,
}

impl<'a> AliasNameUniquer<'a> {
    #[inline]
    pub fn new(uniquer: &'a mut NameUniquer) -> Self {
        Self {
            uniquer,
            aliases: HashMap::new(),
        }
    }
    pub fn make_alias(&mut self, name: &str) -> &str {
        if !self.aliases.contains_key(name) {
            let result = self.uniquer.add_name(name.to_string());
            self.aliases.insert(name.to_string(), result);
        }
        match self.aliases.get(name) {
            Some(refer) => refer,
            None => unreachable!(),
        }
    }
    pub fn get_alias(&self, name: &str) -> Option<&str> {
        self.aliases.get(name)
            .map(|a| a.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_test() {
        let mut n = NameUniquer::new();
        let name = String::from("result");
        assert_eq!(name, n.add_name(name.clone()));
        assert_eq!(format!("{}_0", name), n.add_name(name.clone()));
        assert_eq!(format!("{}_1", name), n.add_name(name.clone()));
        {
            let mut a = AliasNameUniquer::new(&mut n);
            assert_eq!(
                a.make_alias(&name).to_string(),
                a.make_alias(&name).to_string()
            );
            let other_name = "input";
            assert_eq!(
                a.make_alias(other_name).to_string(),
                a.make_alias(other_name).to_string()
            );
        }
    }
}
