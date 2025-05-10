use std::io::{self, Write};
use crate::configuration;
use crate::openrouter;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Model {
    id: String,
    name: String,
}

pub async fn handle_help(command: &str) -> Result<(), Box<dyn std::error::Error>> {
  
    println!("@set-model should set model");
    Ok(())
}