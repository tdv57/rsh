

// echo
// cd
// exit
// pwd
// export
// history
// rsh

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::thread::spawn;
use tokio::sync::Mutex;


use crate::command_handler::handler::CommandExecuter::IsSpawn;
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
    pub async fn echo(&self, instruction: Instruction) -> i32 {
        // Je dois respecter l'option -n 
        // Une fois que c'est fait j'écris et je lis là c'est demandé
        // Si j'ai stdin je lis args et j'écris dans stdout
        //
        // Echo retourne un \n si il lit depuis un fichier ou un pipe
        let mut buffer = String::new();
        
        match instruction.get_i(){
            &Input::File(_) | &Input::Pipe(_) => {
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


    pub async fn cd(&mut self, instruction: Instruction) -> i32 {
        if instruction.get_len_args() > 1 {
            println!("cd needs 0 or 1 arguments found: {} arguments", instruction.get_len_args());
            return 1;
        }
        let path_name = if instruction.get_len_args() == 0 {
            match self.intern_variables.get("HOME") {
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


    pub async fn exit(&mut self, instruction: Instruction) -> i32 {
        std::process::exit(0);
    }

    pub async fn pwd(&mut self, mut instruction: Instruction) -> i32 {
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
                Output::Pipe(mut pipe) => {
                    pipe.write_all(pwd.as_bytes()).await.expect("ShellVariable::pwd Output::Pipe impossible d'écrire dans le pipe");
                },
                Output::Stdout => {
                    println!("{}", pwd);
                }
            }
        } else {
            panic!("ShellVariable::update_pwd $PWD unset")
        }
        0
    }
    pub async fn export(&mut self, instruction: Instruction) -> i32 {
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
    pub async fn history(&mut self, mut instruction: Instruction) -> i32 {
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
                Output::Pipe(mut pipe) => {
                    pipe.write_all(pwd.as_bytes()).await.expect("ShellVariable::pwd Output::Pipe impossible d'écrire dans le pipe");
                },
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
    pub async fn rsh(&self, instruction: Instruction, is_spawn: IsSpawn) -> i32 {
        let cmd = "/root/rust/projet_final/rust_shell/target/debug/rust_shell";

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
                history.push_str(cmd);
                history.push('\n');
            } else {
                panic!("ShellVariable::update_history $HISTORY unset");
            }
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
        Self::init_PATH(&mut shell_variables);
        shell_variables
    }

    pub fn init_PWD(shell_variables: &mut HashMap<String, String>) -> () {
        let pwd = std::env::current_dir().expect("ShellVariable::init_PWD impossible de récupérer le chemin").to_string_lossy().into_owned();
        shell_variables.insert("PWD".to_string(), pwd.clone());
        shell_variables.insert("OLDPWD".to_string(), pwd);
    }

    pub fn init_HISTORY(shell_variables: &mut HashMap<String, String>) -> () {
        shell_variables.insert("HISTORY".to_string(),String::new());
    }

    pub fn init_USER(shell_variables: &mut HashMap<String, String>) -> () {
        let user = match std::env::var("USER") {
            Ok(u) => u,
            Err(u) => "Unknown".to_string(),
        };
        shell_variables.insert("USER".to_string(), user);
    }

    pub fn init_SHELL(shell_variables: &mut HashMap<String, String>) -> () {
        shell_variables.insert("SHELL".to_string(), "rsh".to_string());
    }

    pub fn init_last_status(shell_variables: &mut HashMap<String, String>) -> () {
        shell_variables.insert("?".to_string(), "0".to_string());
    }

    pub fn init_PATH(shell_variables: &mut HashMap<String, String>) -> (){
        shell_variables.insert("PATH".to_string(), String::new());
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
        println!("{}", home);
        let file = OpenOptions::new()
                    .read(true)
                    .write(true)   // nécessaire avec create(true)
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


    pub async fn exec_instruction(&mut self, instruction : Instruction, is_spawn: IsSpawn) -> i32 {
        let cmd = instruction.get_command();
        let mut res = 0;
        if let Some(&shell_command) = SHELL_COMMANDS.get(cmd.as_str()) {
            return match shell_command {
                
                ShellCommands::CD => self.cd(instruction).await,
                ShellCommands::ECHO => self.echo(instruction).await,
                ShellCommands::EXIT => self.exit(instruction).await,
                ShellCommands::EXPORT => self.export(instruction).await,
                ShellCommands::HISTORY => self.history(instruction).await,
                ShellCommands::PWD => self.pwd(instruction).await,
                ShellCommands::RSH => self.rsh(instruction, is_spawn).await,
            };
        }
        CommandExecuter::exec_instruction(instruction).await
    }





}

pub mod InternalCommand {


}

