
use std::fmt::Display;
use std::error::Error;
use std::collections::HashMap;
use command_handler::handler::CommandHandler;

use crate::shell_instance::ShellRunning;


pub mod shell_error;
pub mod token;
pub mod input;
pub mod output;
pub mod command_handler;
pub mod instruction;
pub mod shell_variables;
pub mod shell_instance;
pub mod io_manager;


#[tokio::main]
async fn main() -> () {
//     // peut être mettre dans un objet
//     // Créer un objet parser qui prend la ligne et la parse;
    ShellRunning::run(None).await;
    


}