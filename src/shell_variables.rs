

// echo
// cd
// exit
// pwd
// export
// history
// rsh

use std::collections::HashMap;
use std::env::var;
use std::env::vars_os;
use std::hash::Hash;
use std::sync::Arc;
use std::thread::spawn;
use tokio::sync::Mutex;


use crate::command_handler::handler::CommandExecuter::IsSpawn;
use crate::instruction;
use crate::instruction::*;
use crate::command_handler::handler::CommandExecuter;
use crate::output::*;
use crate::input::*;
use crate::shell_instance;
use crate::shell_instance::ShellInstance;
use crate::shell_variables;
use std::pin::Pin;

use std::path::{Path, PathBuf};
use std::env;

#[cfg(unix)]
use nix::unistd::{fork, ForkResult, dup2};
#[cfg(unix)]
use nix::sys::wait::waitpid;
use std::os::unix::io::AsRawFd;

use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use once_cell::sync::Lazy;
use tokio::io::{AsyncReadExt, BufReader};

use crate::token::*;
use crate::shell_error::ShellError;

#[derive(Clone, Copy)]
pub enum ShellCommands {
    ECHO,
    CD,
    EXIT,
    PWD,
    EXPORT,
    HISTORY,
    RSH
}

static SHELL_COMMANDS: Lazy<HashMap<&'static str, ShellCommands>> = Lazy::new(|| {
    let mut shell_commands = HashMap::new();
    shell_commands.insert("echo", ShellCommands::ECHO);
    shell_commands.insert("cd", ShellCommands::CD);
    shell_commands.insert("exit", ShellCommands::EXIT);
    shell_commands.insert("pwd", ShellCommands::PWD);
    shell_commands.insert("export", ShellCommands::EXPORT);
    shell_commands.insert("history", ShellCommands::HISTORY);
    shell_commands.insert("rsh", ShellCommands::RSH);
    shell_commands
});

#[derive(Clone)]
pub struct ShellVariables {
    extern_variables: HashMap<String, String>,
    intern_variables: HashMap<String, String>,
    shell_variables: HashMap<String, String>,
}

impl ShellVariables {
    pub async fn echo(&self, instruction: &mut Instruction) -> i32 {
        // Je dois respecter l'option -n 
        // Une fois que c'est fait j'écris et je lis là c'est demandé
        // Si j'ai stdin je lis args et j'écris dans stdout
        //
        // Echo retourne un \n si il lit depuis un fichier ou un pipe
        let mut buffer = String::new();
        
        match instruction.get_i(){
            &Input::File(_) | &Input::Pipe => {
                buffer.push('\n');
            }, 
            &Input::Stdin => {
                let args = instruction.get_args();
                if args.is_empty() {
                    buffer.push('\n');
                } else {
                    let option_n = args[0].trim() == "-n";
                    if !(option_n) {
                        buffer.push_str(&args[0]);
                        buffer.push(' ');
                    }

                    for i in 1..args.len() {
                            buffer.push_str(&args[i]);
                            buffer.push(' ');
                        }

                    if !(option_n) {
                        buffer.push('\n');
                    }
                }
            }
        };
        Output::write(instruction, buffer).await;
        0
    }


    pub async fn cd(&mut self, instruction: &mut Instruction) -> i32 {
        if instruction.get_len_args() > 1 {
            println!("cd needs 0 or 1 arguments found: {} arguments", instruction.get_len_args());
            return 1;
        }
        let path_name = if instruction.get_len_args() == 0 {
            match self.shell_variables.get("HOME") {
                Some(home) => home, 
                None => {println!("cd: $HOME not set"); return 1;}
            }
        } else {
            &instruction.get_args()[0]
        };

        let path = Path::new(path_name);

        if let Err(_) = env::set_current_dir(path) {
            println!("cd: unknown directory: {}", path_name);
            return 1;
        }
        match env::current_dir() {
            Ok(current_dir) => {
                self.update_pwd(current_dir);
            }
            Err(_) => {
                println!("cd: failed to get current directory");
                return 1;
            }
        }
        0
    }


    pub async fn exit(&mut self, instruction: &mut Instruction) -> i32 {
        std::process::exit(0);
    }

    pub async fn pwd(&mut self, mut instruction: &mut Instruction) -> i32 {
        if let Some(pwd) = self.shell_variables.get("PWD") {
            match instruction.take_o_put_stdout() {
                Output::FileAppend(file) => {
                    let mut file_open = OpenOptions::new()
                                                        .create(true)
                                                        .append(true)
                                                        .open(file)
                                                        .await
                                                        .expect("ShellVariable::pwd Output::FileAppend impossible d'ouvrir le fichier");
                    file_open.write_all(pwd.as_bytes()).await.expect("ShellVariable::pwd Output::FileAppend impossible d'écrire");
                },
                Output::FileOverwrite(file) => {
                    let mut file_open = OpenOptions::new()
                                                     .create(true)
                                                     .write(true)
                                                     .open(file)
                                                     .await
                                                     .expect("ShellVariable::pwd Output::OverWrite impossible d'ouvrir le fichier");
                    file_open.write_all(pwd.as_bytes()).await.expect("ShellVariable::pwd Output::OverWrite impossible d'écrire");
                },
                Output::Pipe => (),
                Output::Stdout => {
                    println!("{}", pwd);
                }
            }
        } else {
            panic!("ShellVariable::update_pwd $PWD unset")
        }
        0
    }
    pub async fn export(&mut self, instruction: &mut Instruction) -> i32 {
        // On va faire un export différent en gros le premier = est considéré comme affectation
        // ensuite il faut rencontrer un \ pour terminer l'affectation
        // 
        let all_affectations = instruction.get_args().join(" ");
        let affectations: Vec<&str> = all_affectations.split(" \\ ").collect();
        for affectation in affectations {
            let mut var = String::new();
            let mut value = String::new();
            let mut parsing_var = true;
            for c in affectation.chars() {
                if c=='=' && parsing_var {
                    if var.is_empty() {
                        break;
                    }
                    parsing_var = false;
                } else if parsing_var {
                    var.push(c);
                } else if !parsing_var {
                    value.push(c);
                }
            }
            var = var.trim().to_string();
            value = value.trim().to_string();
            if !var.is_empty() && !value.is_empty() {
                self.intern_variables.insert(var.clone(), value.clone());
                self.extern_variables.remove(&var);
            } else if !var.is_empty() && value.is_empty() {
                if let Some(extern_value) = self.extern_variables.remove(&var) {
                    self.intern_variables.insert(var.clone(), extern_value.clone());
                }
            }
        }
        0
    }
    pub async fn history(&mut self, mut instruction: &mut Instruction) -> i32 {
        if let Some(pwd) = self.shell_variables.get("HISTORY") {
            match instruction.take_o_put_stdout() {
                Output::FileAppend(file) => {
                    let mut file_open = OpenOptions::new()
                                                        .create(true)
                                                        .append(true)
                                                        .open(file)
                                                        .await
                                                        .expect("ShellVariable::pwd Output::FileAppend impossible d'ouvrir le fichier");
                    file_open.write_all(pwd.as_bytes()).await.expect("ShellVariable::pwd Output::FileAppend impossible d'écrire");
                },
                Output::FileOverwrite(file) => {
                    let mut file_open = OpenOptions::new()
                                                     .create(true)
                                                     .write(true)
                                                     .open(file)
                                                     .await
                                                     .expect("ShellVariable::pwd Output::OverWrite impossible d'ouvrir le fichier");
                    file_open.write_all(pwd.as_bytes()).await.expect("ShellVariable::pwd Output::OverWrite impossible d'écrire");
                },
                Output::Pipe => (),
                Output::Stdout => {
                    println!("{}", pwd);
                }
            }
        } else {
            panic!("ShellVariable::update_pwd $PWD unset")
        }
        0
    }

    // Je dois envoyer les variables internes mais pas les externes 
    // Je dois rediriger l'output et l'input
    pub async fn rsh(&self, instruction: &mut Instruction, is_spawn: IsSpawn) -> i32 {
        let cmd = "/bin/rust_shell";

        let mut command = tokio::process::Command::new(cmd);

        match is_spawn {
            IsSpawn::SPAWN => {
                command.stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());

                let _child = command.spawn().expect("ShellVariables::rsh error to run child process mode SPAWN");
                0 
            },
            IsSpawn::NOTSPAWN => {
                let status = command.spawn()
                                    .expect("ShellVariables::rsh error to run child process mode NOTSPAWN")
                                    .wait()
                                    .await
                                    .expect("ShellVariables::rsh error to wait child process mode NOTSPAWN");

                status.code().unwrap_or(1)
            }
        }
    }

    pub fn update_pwd(&mut self, new_path: PathBuf ) -> () {
        if let Some(pwd) = self.shell_variables.get("PWD") {
            self.shell_variables.insert("OLDPWD".to_string(), pwd.clone());
            self.shell_variables.insert("PWD".to_string(), new_path.display().to_string());
        } else {
            panic!("ShellVariable::update_pwd $PWD unset");
        }
    }

    pub fn get_pwd(&self) -> &str {
        if let Some(pwd) = self.shell_variables.get("PWD") {
            return pwd;
        } else {
            panic!("ShellVariable::get_pwd $PWD unset");
        }
    }

    pub fn get_old_pwd(&self) -> &str {
        if let Some(old_pwd) = self.shell_variables.get("OLDPWD") {
            return old_pwd;
        } else {
            panic!("ShellVariable::get_old_pwd $OLDPWD unset");
        }
    }

    pub fn get_user(&self) -> &str {
        if let Some(user) = self.shell_variables.get("USER") {
            return user;
        } else {
            panic!("ShellVariable::get_user $USER unset");
        }
    }

    pub fn update_history(&mut self, cmd : &str) -> () {
        if !cmd.is_empty() {
            if let Some(mut history) = self.shell_variables.get_mut("HISTORY") {
                history.push_str(cmd.trim());
                history.push('\n');
            } else {
                panic!("ShellVariable::update_history $HISTORY unset");
            }
        }
    }

    pub fn get_history(&self ) -> Vec<String> {
        if let Some(history_str) = self.shell_variables.get("HISTORY") {
            history_str
                .lines()                  
                .filter(|line| !line.is_empty()) 
                .rev()
                .map(|line| line.to_string())    
                .collect()
        } else {
            panic!("ShellVariables::get_history HISTORY unset"); 
        }
    }

    pub fn update_status(&mut self, status: i32) -> () {
        self.shell_variables.insert("?".to_string(), status.to_string());
    }

    pub fn get_status(&self) -> i32 {
        if let Some(status) = self.shell_variables.get("?") {
            return status.parse().unwrap();
        } else {
            panic!("ShellVariables::get_status ? unset");
        }
    }

    pub async fn new(some_intern_variables : &Option<HashMap<String, String>>) -> Self {
        // Initialiser toutes les variables
        // Variables internes lire -> .rshrc
        let mut intern_variables: HashMap<String, String> = HashMap::new();
        if  let Some(temp_intern_variables) = some_intern_variables {
            intern_variables = temp_intern_variables.clone();
        } else {
            intern_variables = Self::init_intern_variables().await;
        }
        let mut extern_variables: HashMap<String, String> = HashMap::new();
        let mut shell_variables: HashMap<String, String> = Self::init_shell_variables();
        Self {intern_variables, extern_variables, shell_variables}
    }

    pub fn init_shell_variables() -> HashMap<String, String> {
        let mut shell_variables: HashMap<String, String> = HashMap::new();
        Self::init_PWD(&mut shell_variables);
        Self::init_HISTORY(&mut shell_variables);
        Self::init_SHELL(&mut shell_variables);
        Self::init_USER(&mut shell_variables);
        Self::init_last_status(&mut shell_variables);
        Self::init_STATUS(&mut shell_variables);
        Self::init_HOME(&mut shell_variables);
        shell_variables
    }

    pub fn init_STATUS(shell_variables: &mut HashMap<String, String>) -> () {
        shell_variables.insert("?".to_string(), "0".to_string());
    }

    pub fn init_HOME(shell_variables: &mut HashMap<String, String>) -> () {
        if let Ok(home) = std::env::var("HOME") {
            shell_variables.insert("HOME".to_string(), home.clone());
        }
    }

    pub fn init_PWD(shell_variables: &mut HashMap<String, String>) -> () {
        let pwd = std::env::current_dir().expect("ShellVariable::init_PWD impossible de récupérer le chemin").to_string_lossy().into_owned();
        shell_variables.insert("PWD".to_string(), pwd.clone());
        shell_variables.insert("OLDPWD".to_string(), pwd);
    }

    pub fn init_HISTORY(shell_variables: &mut HashMap<String, String>) -> () {
        shell_variables.insert("HISTORY".to_string(),String::new());
    }

    pub fn init_USER(intern_variables: &mut HashMap<String, String>) -> () {
        let user = match std::env::var("USER") {
            Ok(u) => u,
            Err(u) => "Unknown".to_string(),
        };
        intern_variables.insert("USER".to_string(), user);
    }

    pub fn init_SHELL(shell_variables: &mut HashMap<String, String>) -> () {
        shell_variables.insert("SHELL".to_string(), "rsh".to_string());
    }

    pub fn init_last_status(shell_variables: &mut HashMap<String, String>) -> () {
        shell_variables.insert("?".to_string(), "0".to_string());
    }

    pub fn init_PATH(intern_variables: &mut HashMap<String, String>) -> (){
        intern_variables.insert("PATH".to_string(), String::from("/bin:"));
    }

    pub fn parse_rshrc(rshrc: String, intern_variables: &mut HashMap<String, String>) -> () {
        for line in rshrc.split('\n') {
            if let Some((key, value)) = line.split_once('=') {
                intern_variables.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }

    pub async fn init_intern_variables() -> HashMap<String, String> {
        // doit lire dans ~/.rshrc  
        let mut intern_variables = HashMap::new();
        let mut home = match env::var("HOME") {
            Err(e) => panic!("HOME var empty"),
            Ok(home) => home,
        };
        home.push_str(&String::from("/.rshrc"));
        //println!("{}", home);
        let file = OpenOptions::new()
                    .read(true)
                    .write(true)   
                    .create(true)
                    .open(home)
                    .await
                    .expect("issue while opening .rshrc");
        let mut buf = String::new();
        let mut reader = BufReader::new(file);
        reader.read_to_string(&mut buf).await.expect("ShellVariable::init_intern_variables issue while reading the file .rshrc");
        Self::parse_rshrc(buf, &mut intern_variables);
        intern_variables
    }

    pub fn look_for_file_or_dir_starting_with(&self, cmd: &str) -> Vec<String> {
        let mut candidates = Vec::new();
        let mut current_dir = PathBuf::new();
        let (current_dir, file_or_dir) = if cmd.contains('/') {
            let mut current_dir_str = String::new();
            let partial_path: Vec<&str> = cmd.split('/').collect();
            for path in &partial_path[..partial_path.len()-1] {
                current_dir_str.push_str(path);
                current_dir_str.push('/');
            }
            current_dir = PathBuf::from(current_dir_str);
            (current_dir, partial_path[partial_path.len()-1])

        } else {
            current_dir = PathBuf::from(".");
            (current_dir, cmd)
        };
        
        if let Ok(entries) = std::fs::read_dir(current_dir.clone()) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(file_or_dir) {
                        let path = current_dir.join(name);
                        let candidate = if let Ok(file_type) = entry.file_type() {
                                if file_type.is_dir() {
                                    format!("{}/", path.to_string_lossy()) 
                                } else {
                                    path.to_string_lossy().to_string()
                                }
                        } else {
                            path.to_string_lossy().to_string()
                        };
                        candidates.push(candidate);
                    }
                }
            }
        }
        candidates
    }

    pub fn look_for_path_starting_with(&self, cmd: &str) -> Vec<String>{
        let mut candidates = Vec::new();
        let PATH = match self.look_into_variables("PATH") {
                Some(path) => path,
                None => panic!("ShellVariables::look_for_path PATH unset"),
            };
        for dir in PATH.split(':') {
            let dir = Path::new(dir);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with(cmd) {
                            candidates.push(name.to_string())
                        }
                    }
                }
            }
        }
        candidates
    }

    pub fn look_for_path(&self, cmd: &str) -> Result<String, ShellError> {
        if !cmd.contains('/') {
            let PATH = match self.look_into_variables("PATH") {
                Some(path) => path,
                None => panic!("ShellVariables::look_for_path PATH unset"),
            };
            for dir in PATH.split(':') {
                let full_path = Path::new(dir).join(cmd);
                if full_path.is_file() {
                    let mut response = dir.to_string();
                    response.push('/');
                    response.push_str(cmd);
                    return Ok(response);
                }
            }
            let mut response = "command ".to_string();
            response.push_str(cmd);
            response.push_str(" not found");
            Err(ShellError::CommandError(response))
        } else {
            let full_path = Path::new(cmd);
            if full_path.is_file() {
                return Ok(cmd.to_string());
            } else {
                let mut response = "command ".to_string();
                response.push_str(cmd);
                response.push_str(" not found");
                Err(ShellError::CommandError(response))
            }
        }


    }

    pub async fn exec_instruction(&mut self, instruction : &mut Instruction, is_spawn: IsSpawn) -> Result<tokio::process::Command, i32> {
        let cmd = instruction.get_command();

        let mut res = 0;
        if let Some(&shell_command) = SHELL_COMMANDS.get(cmd.as_str()) {
            if is_spawn == IsSpawn::NOTSPAWN {
                return Err(match shell_command {
                    
                    ShellCommands::CD => self.cd(instruction).await,
                    ShellCommands::ECHO => self.echo(instruction).await,
                    ShellCommands::EXIT => self.exit(instruction).await,
                    ShellCommands::EXPORT => self.export(instruction).await,
                    ShellCommands::HISTORY => self.history(instruction).await,
                    ShellCommands::PWD => self.pwd(instruction).await,
                    ShellCommands::RSH => self.rsh(instruction, is_spawn).await,
                });
            } else {
                return Err(match shell_command {
                    
                    ShellCommands::CD => {self.cd(instruction); 0},
                    ShellCommands::ECHO => {self.echo(instruction); 0},
                    ShellCommands::EXIT => {self.exit(instruction); 0},
                    ShellCommands::EXPORT => {self.export(instruction); 0},
                    ShellCommands::HISTORY => {self.history(instruction); 0},
                    ShellCommands::PWD => {self.pwd(instruction); 0},
                    ShellCommands::RSH => {self.rsh(instruction, is_spawn); 0},
                });
            }

        }
        
        let mut instruction = instruction;
        let cmd: String = ShellError::handle_shell_error(self.look_for_path(&cmd))?;
        instruction.set_command(cmd);

        Ok(CommandExecuter::exec_instruction(instruction,is_spawn).await)
    }

    pub fn look_into_variables(&self, variables: &str) -> Option<&str> {
        if let Some(var_value) = self.shell_variables.get(variables) {
            return Some(var_value);
        } else if let Some(var_value) = self.intern_variables.get(variables) {
            return Some(var_value);
        } else if let Some(var_value) = self.extern_variables.get(variables) {
            return Some(var_value);
        } else {
            return None;
        }
    }

    pub fn push_in_var_or(&self, to_push_in: &mut String, value_to_push: String) ->  Result<(), ShellError> {
        if value_to_push.is_empty() {
            to_push_in.push('$');
            return Ok(());
        } else {
            match self.look_into_variables(&value_to_push) {
                Some(value) => {
                    to_push_in.push_str(value); 
                    return Ok(());
                },
                None => {
                    return Err(ShellError::VarNotFound(value_to_push));
                },
            }
        }
    }

    pub fn parse_variables<'a>(&self, var_name: &mut std::iter::Peekable<std::str::Chars<'a>>, quote: Option<char>) -> Result<String,ShellError> {
        let mut var_value = String::new();
        let mut current_var_value = String::new();
        while let Some(&next_char) = var_name.peek() {
            if Some(next_char) == quote || next_char == ' ' {
                self.push_in_var_or(&mut var_value, current_var_value)?;
                current_var_value = String::new();
                break;
            } else if next_char == '$' {
                self.push_in_var_or(&mut var_value, current_var_value)?;
                current_var_value = String::new();
            } else {
                current_var_value.push(next_char);
            }
            var_name.next();
        }
        if current_var_value.is_empty() {
            var_value.push('$');
        } else {
            self.push_in_var_or(&mut var_value, current_var_value)?;
        }
        Ok(var_value)
    }
    pub fn expand_variables(&self, tokens: Vec<Token>) -> Result<Vec<Token>,ShellError> {
        tokens.into_iter().map(|token| -> Result<Token, ShellError>{
            match token {
                Token::NotOperator(TokenNotOperator::Variable(var_name)) => {
                    let mut chars = var_name.chars().peekable();
                    match self.parse_variables(&mut chars, None) {
                        Ok(var_value) => {return Ok(Token::get_command(var_value.to_string()));},
                        Err(e) => return Err(e),
                    }
                },
                Token::NotOperator(TokenNotOperator::Inquote(in_quote)) => {
                    let mut chars = in_quote.chars().peekable();
                    let first_char = chars.next();
                    if first_char == Some('\'') {
                        return Ok(Token::NotOperator(TokenNotOperator::Inquote(in_quote)));
                    } else {
                        let mut new_quote = String::new();
                        new_quote.push('"');
                        while let Some(next_char) = chars.next() {
                            if next_char == '$' {
                                let var_value = self.parse_variables(&mut chars, Some('"'))?;
                                new_quote.push_str(&var_value);
                            } else {
                                new_quote.push(next_char);
                            }
                        }
                        return Ok(Token::NotOperator(TokenNotOperator::Inquote(new_quote)));
                    }
                    // On pourra faire des plus compliqués ensuite en fonction de si on a "" ou '' au début
                    // si ' on fait rien 
                    // si " on change"
                },
                _ => Ok(token),
            }
        }).collect()
    }


}


