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

    fn print_new_command(current_command: &StringIndex, user: &str) {
        print!("\r\x1b[2K"); 
        let mut to_prompt = current_command.get().to_string();
        to_prompt.push(' ');
        print!("{}{}", user, to_prompt);
        let move_cursor = to_prompt.len() - current_command.get_index();
        print!("\x1b[{}D", move_cursor);
        std::io::stdout().flush().unwrap();
    }

    #[derive(Debug)]
    pub struct StringIndex {
        string: String,
        index: usize
    }

    impl StringIndex {
        pub fn push_str(&mut self, c: &str) {
            self.string.push_str(c);
        }

        pub fn insert(&mut self, c: char) {
            self.string.insert(self.index, c);
            self.index += 1;
        }

        pub fn insert_str(&mut self, c: &str) {
            for char in c.chars() {
                self.string.insert(self.index, char);
                self.index+=1;
            }
        }

        pub fn get_index(&self) -> usize {
            self.index
        }
        pub fn remove(&mut self) {
            if self.index > 0 {
              self.string.remove(self.index-1);
              self.index -= 1;
            }
        }
        pub fn new() -> Self {
            Self {string: String::new(), index:0}
        }

        pub fn from(c: &str) -> Self {
            let string = String::from(c);
            let index = c.len();
            Self {string, index}
        }

        pub fn is_empty(&self) -> bool {
            self.string.is_empty()
        }

        pub fn move_right(&mut self) {
            if self.index < self.string.len() {
                self.index += 1;
            }
        }

        pub fn move_left(&mut self) {
            if self.index > 0 {
                self.index -= 1;
            }
        }


        pub fn set(&mut self, c: String) -> () {
            self.index = c.len();
            self.string = c;
        }

        pub fn get(&self) -> &str {
            &self.string
        }

    }
    pub async fn handle_command(shell_variables: &mut ShellVariables, user: &str) -> String {
        


        let history = shell_variables.get_history();
        let mut current_command = StringIndex::new();
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
                            if current_command.is_empty() {
                                println!("^D");
                                print!("\r\x1b[2D");
                                stdout.flush().unwrap();
                                disable_raw_mode().unwrap();
                                std::process::exit(0);
                            }
                        }
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            current_command.insert_str("^C");
                            print!("{}","^C");
                            break;
                        }
                        (KeyCode::Char(c), _) => {
                            current_command.insert(c);
                            if history_index==0 {new_command.push(c);}
                            print_new_command(&current_command, user);
                        }
                        (KeyCode::Enter, _) => {
                            current_command.push_str("\n");
                            print!("{}",'\n');
                            break;
                        }
                        (KeyCode::Backspace, _) => {
                            current_command.remove();
                            print_new_command(&current_command, user);        
                        }
                        (KeyCode::Up, _)  => {
                            if history_index < history.len() {
                                history_index += 1;
                                current_command.set(history[history_index-1].clone());
                            }
                            print_new_command(&current_command, &user);
                        }
                        (KeyCode::Down, _) => {
                            if history_index>0 {
                                history_index-=1;
                                if history_index == 0 {
                                    current_command.set(new_command.clone());
                                } else {
                                    current_command.set(history[history_index-1].clone());
                                }
                                print_new_command(&current_command, &user);
                            } 
                        }
                        (KeyCode::Left, _) => {
                            current_command.move_left();
                            print_new_command(&current_command, user);
                        }
                        (KeyCode::Right, _) => {
                            current_command.move_right();
                            print_new_command(&current_command, user);
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
        
        current_command.get().to_string()
    }
}
