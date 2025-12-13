// Le but c'est que main initialise les variables d'environnement mais qu'ensuite si je fais rsh
// Je lance le shell avec les variables enfants copiées

use crate::shell_instance;
use crate::shell_variables;
use crate::token::*;
use crate::input::*;
use crate::output::*;
use crate::instruction::*;
use crate::io_manager::read_line_sync;
use crate::shell_error::*;
use crate::command_handler::handler::*;
use crate::shell_variables::ShellVariables;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ShellInstance {
    shell_variables: Arc<Mutex<ShellVariables>>,
}

impl ShellInstance {

    pub async fn new() -> Self {
        let shell_variables = Arc::new(Mutex::new(ShellVariables::new(&None).await)); // crée ton struct normalement
        Self { shell_variables }
    }

    pub async fn from(shell_instance: Option<&ShellInstance>) -> ShellInstance{
        match shell_instance {
            None => Self::new().await,
            Some(rsh) => rsh.clone()
        }
    }

    pub async fn from_shell_variables(shell_variables: Arc<Mutex<ShellVariables>>) -> ShellInstance {
        Self { shell_variables: shell_variables.clone()}
    }
    // Mettre ça autre part dans un wrapper
    pub async fn handle_command(&mut self, command: String) -> Result<(), i32> {
        // Avoir une instance du shell
        if command.is_empty() {return Err(0);};
        let mut tokens = ShellError::handle_shell_error(CommandParser::get_token(&command))?;

        tokens = ShellError::handle_shell_error(CommandParser::check_commands(tokens))?;
        let divided_tokens = CommandParser::divide_tokens(tokens);
        let mut status = 0;
        for divided_token in divided_tokens {
            let expanded_token = ShellError::handle_shell_error(CommandParser::expand_variables(divided_token))?;
            let instructions_or_tokens = CommandParser::build_instructions(expanded_token);
            status = CommandExecuter::execute(self.shell_variables.clone(), instructions_or_tokens).await;
            
            // Je dois directement exécuter
        }
        Err(status)
    }

    pub async fn get_command(&mut  self, command: &str) -> Result<(), i32> {
        // mettre à jour l'historique  
        // la lancer
        if command.is_empty() {return Err(0);}; 
        {
          let mut shell_variables_locked = self.shell_variables.lock().await;
          shell_variables_locked.update_history(command);
        }
        match self.handle_command(command.to_string()).await {
            Err(status) => Err(status),
            Ok(_) => panic!("ShellInstance::get_command handle_command should not return Ok"),
        }
    }

    pub async fn print_newline(&self) -> String {
        let shell_variables_locked = self.shell_variables.lock().await;
        let user = shell_variables_locked.get_user();
        let pwd=  shell_variables_locked.get_pwd();
        let mut user = String::from(user);
        user.push_str(":");
        user.push_str(pwd);
        user.push_str("# ");
        read_line_sync(&user)
    }
}

pub mod ShellRunning {
    use crate::{io_manager::{IOManager, read_line_sync}, shell_instance::{self, ShellInstance}, shell_variables::{self, ShellCommands}};

    pub async fn run(shell_instance: Option<&ShellInstance>) -> std::io::Result<()>{
        let mut shell_instance = ShellInstance::from(shell_instance).await;
        println!("Voici la sortie de la commande !");
        loop {
            println!("Dans loop");
            let command = shell_instance.print_newline().await;  
            match shell_instance.get_command(&command).await {
                Ok(_) => {
                    panic!("ShellRunning::run receive Ok result after get_command");
                }
                Err(e) => {
                    eprintln!("exit code: {:?}", e);
                }
            }
        }
        Ok(())
    }
}