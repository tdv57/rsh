// ctrl C
// ctrl D
// up arrow
// down arrow
pub mod SignalHandler {
    use std::io::{self, Write};
    use tokio::process::Child;
    use nix::sys::signal::{Signal};
    use nix::libc::{kill};
    use nix::unistd::Pid;
    use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
    use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

    use crate::shell_variables::{self, ShellVariables};

    pub async fn handle_ctrl_c(child: &mut Child) -> i32 {
        let child_id: i32 = child.id().expect("SignalHandler::handle_ctrl_c child process has no id") as i32;
        tokio::select! {
            status = child.wait() => {
                match status {
                    Ok(status) => status.code().unwrap_or(1),
                    Err(_) => panic!("SignalHandler::handle_ctrl_c issue while waiting for child"),
                }

            }
            _ = tokio::signal::ctrl_c() => {
                unsafe { kill(child_id, Signal::SIGINT as i32);}
                print!("^C");
                return 2;
            }
        }
    }

    fn print_new_command(current_command: &str, user: &str) {
        print!("\r\x1b[2K"); 
        print!("{}{}", user, current_command);
        std::io::stdout().flush().unwrap();
    }

    pub async fn handle_command(shell_variables: &mut ShellVariables, user: &str) -> String {
        


        let history = shell_variables.get_history();
        let mut current_command = String::new();
        let mut new_command = String::new();
        let mut history_index = 0;
        let min_index = user.len();
        let mut max_index = user.len();
        let mut stdout = std::io::stdout();
        stdout.flush().unwrap();
        match enable_raw_mode() {
            Ok(_) => (),
            Err(_) => panic!("SignalHandle::handle_command issue while passing in raw mode"),
        };
        print!("{}",user);
        stdout.flush().unwrap();
        loop {
            if event::poll(std::time::Duration::from_millis(10)).unwrap() {
                match event::read().unwrap() {
                    Event::Key(KeyEvent { code, modifiers, .. }) => match (code, modifiers) {
                        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                            if current_command == "" {
                                println!("^D");
                                print!("\r\x1b[2D");
                                stdout.flush().unwrap();
                                disable_raw_mode().unwrap();
                                std::process::exit(0);
                            }
                        }
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            current_command.push_str("^C");
                            print!("{}","^C");
                            break;
                        }
                        (KeyCode::Char(c), _) => {
                            current_command.push(c);
                            if history_index==0 {new_command.push(c);}
                            print!("{}", c);
                            stdout.flush().unwrap();
                        }
                        (KeyCode::Enter, _) => {
                            current_command.push('\n');
                            print!("{}",'\n');
                            break;
                        }
                        (KeyCode::Backspace, _) => {
                            if !current_command.is_empty() {
                                if current_command.pop().is_some() {
                                    if history_index == 0 {
                                        new_command.pop();
                                    }
                                    print!("\x08 \x08"); // efface le dernier caractère
                                    stdout.flush().unwrap();
                                }
                            }

                        }
                        (KeyCode::Up, _)  => {
                            if history_index < history.len() {
                                history_index += 1;
                                current_command = history[history_index-1].clone();
                            }
                            print_new_command(&current_command, &user);
                        }
                        (KeyCode::Down, _) => {
                            if history_index>0 {
                                history_index-=1;
                                if history_index == 0 {
                                    current_command = new_command.clone();
                                } else {
                                    current_command = history[history_index-1].clone();
                                }
                                print_new_command(&current_command, &user);
                            } 
                        }
                        (KeyCode::Left, _) => {
                            
                        }
                        (KeyCode::Right, _) => {
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        print!("\r");
        stdout.flush().unwrap();
        disable_raw_mode().unwrap();
        
        current_command
    }
}
