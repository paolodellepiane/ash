use crate::prelude::*;
use pest::Parser;
use pest_derive::Parser;
use std::{collections::HashMap, path::Path};

#[derive(Parser)]
#[grammar = "parsers/pegs/ini.pest"]
pub struct IniParser;

pub fn parse_ini(content: &str) -> Result<HashMap<String, HashMap<String, String>>> {
    // let _guard = stopwatch("ini parse");
    let res = IniParser::parse(Rule::file, content)?.next().unwrap();
    let mut profiles: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section = String::from("");
    profiles.entry(current_section.clone()).or_default();
    for line in res.into_inner() {
        match line.as_rule() {
            Rule::section => {
                current_section = line.into_inner().next().unwrap().as_str().to_string();
                profiles.entry(current_section.clone()).or_default();
            }
            Rule::property => {
                let rules = &mut line.into_inner();
                let name = rules.next().unwrap().as_str();
                let value = rules.next().unwrap().as_str().to_string();
                profiles.get_mut(&current_section).unwrap().insert(name.to_lowercase(), value);
            }
            _ => (),
        }
    }
    Ok(profiles)
}

pub fn parse_ini_from_file(
    path: impl AsRef<Path>,
) -> Result<HashMap<String, HashMap<String, String>>> {
    let content = std::fs::read_to_string(path)?;
    parse_ini(&content)
}

#[cfg(test)]
mod tests {
    const INI: &str = r#"
    [default]
    region = eu-west-1
    
    [profile test]
    role_arn =arn:aws:iam::123123123:role/DescribeInstances
    source_profile = default
    region = eu-west-1
    
    [profile prod]
    role_arn = arn:aws:iam::123123123:role/DescribeInstances
    source_profile = default
    region = eu-west-1
      
"#;

    #[test]
    fn parse_ini_succeeds() {
        let res = super::parse_ini(INI);
        match res {
            Ok(r) => assert_eq!(r.len(), 4),
            Err(err) => panic!("{err:#}"),
        }
    }
}
