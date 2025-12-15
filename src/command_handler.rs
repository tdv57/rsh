pub mod handler {

    use crate::instruction::*;
    use crate::token::*;


    #[derive(Debug)]
    pub enum InstructionOrToken {
        Instruction(Instruction),
        Token(Token),
    }


    pub mod CommandHandler {


    }


    pub mod CommandParser {

        use nix::libc::int32_t;

        use crate::shell_error::*;
        use crate::token::*;
        use crate::input::*;
        use crate::output::*;
        use crate::instruction::*;
        use crate::command_handler::handler::*;


        pub fn get_token(command: &str) -> Result<Vec<Token>, ShellError> {
            let mut chars = command.chars().peekable();
            let mut all_tokens: Vec<Vec<Token>> = Vec::new();
            let mut tokens: Vec<Token> = Vec::new();
            //1 doit commencer par une commande
            let mut command = String::new();
            let mut var = String::new();
            let mut inquote = String::new();
            if let Some(&first_char) = chars.peek() {
                if (is_operator(first_char)) {
                    return Err(ShellError::CommandFirst);
                }

            }

            while let Some(char) = chars.next() {
                if is_operator(char) {
                    if !command.trim().is_empty() {
                    tokens.push(Token::get_command(command.clone().trim().to_string()));
                    command.clear();
                    }

                    match char {
                        '>' => {
                                if chars.peek()==Some(&'>') {
                                    chars.next();
                                    tokens.push(Token::get_redirection_output_append());
                                } else {
                                    tokens.push(Token::get_redirection_output_overwrite());
                                }
                        } ,

                        '<' => tokens.push(Token::get_redirection_input()),

                        '|' => {
                            if chars.peek() == Some(&'|') {
                                chars.next();
                                tokens.push(Token::get_or())
                            } else {
                                tokens.push(Token::get_pipe())
                            }
                        },

                        '&' => {
                            if chars.peek() == Some(&'&') {
                                chars.next();
                                tokens.push(Token::get_and())
                            } else {
                                tokens.push(Token::get_background())
                            }
                        },

                        ';' => tokens.push(Token::get_semi_colon()),
                    _ => {},
                    }
                }
                else if is_var(char) {
                    if !command.trim().is_empty() {
                        tokens.push(Token::get_command(command.clone().trim().to_string()));
                        command.clear();
                    }

                    while let Some(&c) = chars.peek() {
                        if is_command(c) || c=='$' {
                            var.push(c);
                            chars.next();
                        } else if c == ' '{
                            break;
                        } else {
                            break;
                        }
                    }

                    if var.is_empty() {
                        tokens.push(Token::get_command("$".to_string()));
                    } else {
                        tokens.push(Token::get_variable(var.clone().trim().to_string()));
                        var.clear();
                    }
                } else if is_quote(char) {
                    if !command.trim().is_empty() {
                        tokens.push(Token::get_command(command.clone().trim().to_string()));
                        command.clear();
                    }

                    inquote.push(char);
                    let mut closed = false;
                    while let Some(&c) = chars.peek() {
                        inquote.push(c);
                        chars.next();
                        if c == char {
                            closed = true;
                            break;
                        }
                    }

                    if !closed {
                        return Err(ShellError::QuoteNotClosed(inquote.clone()));
                    }

                    tokens.push(Token::get_inquote(inquote.clone().trim().to_string()));
                    inquote.clear();

                }  else {
                    if char == ' ' {
                        if !command.trim().is_empty() {
                            tokens.push(Token::get_command(command.clone().trim().to_string()));
                            command.clear();
                        }
                    } else {
                        command.push(char);
                    }                    
                    
                }
            }
            // Erreur je peux avoir un opérateur à la fin
            // if let Some(Token::Operator(_)) = tokens.last() {
            //     return Err(ShellError::CommandAfterOperator);
            // }
            if !command.trim().is_empty() {
                tokens.push(Token::get_command(command.clone().trim().to_string()));
                command.clear();
            }
            Ok(tokens)
        }

        // Doit checker que la première commande est soit > >> ou une commande classique
        pub fn check_commands(tokens: Vec<Token>) -> Result<Vec<Token>, ShellError> {
            let mut peekable_tokens = tokens.iter().peekable();
            while let Some(token) = peekable_tokens.next() {
                if let Token::Operator(_) = token {
                    if let Some(next_token) = peekable_tokens.peek() {
                        if let Token::Operator(_) = **next_token {
                            return Err(ShellError::CommandAfterOperator);
                        }
                    }
                }
            }
            Ok(tokens)
        }

        pub fn divide_tokens(tokens: Vec<Token>) -> Vec<Vec<Token>> {
            let mut tokens_divided: Vec<Vec<Token>> = Vec::new();
            let mut tokens_to_execute: Vec<Token> = Vec::new();
            for token in tokens {
                if Token::is_semi_colon(&token) {
                    tokens_divided.push(tokens_to_execute.clone());
                    tokens_to_execute.clear();
                } else {
                    tokens_to_execute.push(token);
                }
            }
            if (!tokens_to_execute.is_empty()) {
                tokens_divided.push(tokens_to_execute);
            }
            tokens_divided
        }

        pub fn match_braces(to_match: &str) -> Vec<String> {
            if let Some(start) = to_match.find('{') {
                if let Some(end) = to_match[start..].find('}') {
                    let end = start + end;
                    let before = &to_match[..start];
                    let after = &to_match[end + 1..];
                    let inside = &to_match[start + 1..end];

                    let mut possible_targets: Vec<String> = Vec::new();
                    for in_braces in inside.split(',') {
                        let mut in_in_braces = String::new();
                        in_in_braces.push_str(before);
                        in_in_braces.push_str(in_braces);
                        in_in_braces.push_str(after);
                        for possible_target in match_braces(&in_in_braces) {
                            possible_targets.push(possible_target.to_string());
                        }
                    }
                    return possible_targets;
                }
                return vec![to_match.to_string()];
            }
            vec![to_match.to_string()]
        }

        pub fn match_brackets(to_match: &[char], target: &[char]) -> Option<usize> {
            if target.is_empty() {return None;}
            let mut i = 0;
            while i < to_match.len() && to_match[i] != ']' {
                i+=1;
            }

            if i == to_match.len() {return None;}

            let in_brackets = &to_match[1..i];
            if in_brackets.contains(&target[0]) {
                Some(i + 1) // nombre de chars consommés dans le pattern
            } else {
                None
            }
        }
        // A changer plus tard
        pub fn match_regex_expression(to_match: &[char], target: &[char]) -> bool {
            match(to_match.first(), target.first()) {
                (None, None) => true,
                (None, Some(_)) => false,
                (Some('*'), _) => {
                    match_regex_expression(&to_match[1..], target) ||
                    (!target.is_empty() && match_regex_expression(to_match, &target[1..]))
                },
                (Some('?'), Some(_)) => {
                    if target.is_empty() {return false;}
                    match_regex_expression(&to_match[1..], &target[1..])
                },
                (Some('['), _) => {
                    if let Some(index) = match_brackets(to_match, target) {
                        return match_regex_expression(&to_match[index..], &target[1..]);
                    } else {
                        if !target.is_empty() && target[0] == to_match[0] {
                            return match_regex_expression(&to_match[1..], &target[1..]);
                        } else {
                            return false;
                        }
                    }
                },
                (Some(to_match_char), Some(target_char)) => {
                    if to_match_char == target_char {
                        return match_regex_expression(&to_match[1..], &target[1..]);
                    } else {
                        return false;
                    }
                },
                _ => false,
            }
        }

        pub fn has_regex_expression(to_match: &str) -> bool {
            to_match.contains(['[', '{', '*', '?'])
        }

        pub fn expand_command(tokens: Vec<Token>) -> Vec<Token> {
            let mut result = Vec::new();
            for token in tokens {
                match token {
                    Token::NotOperator(TokenNotOperator::Command(var_name)) => {
                        if has_regex_expression(&var_name) {
                            let partial_paths: Vec<&str> = var_name.split('/').filter(|p| !p.is_empty()).collect();
                            let mut candidates = vec![std::path::PathBuf::from(".")];
                            
                            for partial_path in partial_paths {
                                let mut next_candidates = Vec::new();
                                let possible_match = match_braces(partial_path);
                                for candidate in &candidates {
                                    for to_match in &possible_match {
                                        if has_regex_expression(&to_match) {
                                            let mut entries = match std::fs::read_dir(candidate) {
                                                Ok(e) => e,
                                                Err(_) => continue,
                                            };

                                            for entry in entries {
                                                    let entry = match entry {
                                                        Ok(e) => e,
                                                        Err(_) => continue,
                                                    };
                                                    let name = entry.file_name();
                                                    let target = match name.to_str(){
                                                        Some(name) => name,
                                                        None => continue,
                                                    };
                                                    if match_regex_expression(&to_match.chars().collect::<Vec<_>>(),&target.chars().collect::<Vec<_>>(),){
                                                        next_candidates.push(candidate.join(target));
                                                    } 
                                                    

                                                }
                                            } else {
                                                next_candidates.push(candidate.join(to_match));
                                            }
                                    }
                                }
                                candidates = next_candidates;
                            }
                            for candidate in candidates {
                                let candidate_str = candidate.to_string_lossy().into_owned();
                                result.push(Token::get_command(candidate_str));
                            }
                        } else {
                            result.push(Token::NotOperator(TokenNotOperator::Command(var_name)))
                        }
                        
                    },

                    token => result.push(token),
                }
            }
            result
        }

        pub fn build_instructions(tokens: Vec<Token>) -> Vec<InstructionOrToken> {
            // Je dois parcourir les tokens soit c'est une commande et donc je dois merge
            // Soit c'est > >> <  et donc fin de commande + modification de fichier voir commandes derrière
            // Soit c'est | donc j'écrase stdout et je passe à construire la commande suivante ou push le token
            // Soit c'est un opérateur donc je stoppe la commande je la push et je push le Token

            let mut instructions_or_tokens: Vec<InstructionOrToken> = Vec::new();
            let mut instruction = Instruction::new();

            let mut tokens_iter = tokens.into_iter().peekable();

            while let Some(token) = tokens_iter.next() {
                if Token::is_command(&token) || Token::is_inquote(&token) {
                    instruction.add_cmd_or_inquote_token(token);
                } else if let Token::Operator(TokenOperator::Redirection(redirection_token)) = token  {
                    if let Some(next_token) = tokens_iter.next() {
                        TokenRedirection::apply_redirection(&mut instruction, redirection_token, next_token);
                    }
                } else if Token::is_logic(&token) {
                    instructions_or_tokens.push(InstructionOrToken::Instruction(instruction));
                    instruction = Instruction::new();
                    instructions_or_tokens.push(InstructionOrToken::Token(token));
                    // Je stoppe la formation de l'instruction je push l'instruction et je push le token
                } else {
                    panic!("Token must be Logic Redirection, Command or Inquote");
                }
            }
            if !instruction.is_empty() {
                instructions_or_tokens.push(InstructionOrToken::Instruction(instruction));
            }
            instructions_or_tokens
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn check_getCommand_checkCommands_cmd1() {
                let cmd1: String = "ls -l".to_string();

                let tok1_cmd1 = Token::get_command(cmd1.clone());

                let mut tok1 = Vec::new();
                tok1.push(tok1_cmd1);

                assert_eq!(get_token(&cmd1).unwrap(), tok1);
                let check_tok1 = get_token(&cmd1).unwrap();
                assert_eq!(check_commands(check_tok1).unwrap(), tok1);
            }

            #[test]
            fn check_getCommand_checkCommands_cmd2() {
                let cmd2: String = "Bonjour | |".to_string();

                let tok1_cmd2 = Token::get_command("Bonjour".to_string());
                let tok2_cmd2 = Token::get_pipe();
                let tok3_cmd2 = Token::get_pipe();


                let mut tok2  = Vec::new();
                tok2.push(tok1_cmd2);
                tok2.push(tok2_cmd2);
                tok2.push(tok3_cmd2);

                assert_eq!(get_token(&cmd2).unwrap(), tok2);
                let check_tok2 = get_token(&cmd2).unwrap();
                assert_eq!(check_commands(check_tok2), Err(ShellError::CommandAfterOperator));
            }

            #[test]
            fn check_getCommand_checkCommands_cmd3() {
                let cmd3: String = "echo bonjour a tous > \"bonjour a tous\"".to_string();

                let tok1_cmd3 = Token::get_command("echo bonjour a tous".to_string());
                let tok2_cmd3 = Token::get_redirection_output_overwrite();
                let tok3_cmd3 = Token::get_inquote("\"bonjour a tous\"".to_string());

                let mut tok3 = Vec::new();
                tok3.push(tok1_cmd3);
                tok3.push(tok2_cmd3);
                tok3.push(tok3_cmd3);

                assert_eq!(get_token(&cmd3).unwrap(), tok3);
                let check_tok3 = get_token(&cmd3).unwrap();
                assert_eq!(check_commands(check_tok3).unwrap(), tok3 );
            }

            #[test]
            fn check_getCommand_checkCommands_cmd4() {
                let cmd4: String = "ls; ls".to_string();

                let tok1_cmd4 =  Token::get_command("ls".to_string());
                let tok2_cmd4 = Token::get_semi_colon();
                let tok3_cmd4 = Token::get_command("ls".to_string());


                let mut tok4 = Vec::new();
                tok4.push(tok1_cmd4);
                tok4.push(tok2_cmd4);
                tok4.push(tok3_cmd4);

                assert_eq!(get_token(&cmd4).unwrap(), tok4);
                let check_tok4 = get_token(&cmd4).unwrap();
                assert_eq!(check_commands(check_tok4).unwrap(), tok4);
            }

            #[test]
            fn check_divideToken_cmd4() {
                let cmd4: String = "ls; ls".to_string();


                let check_tok4 = get_token(&cmd4).unwrap();
                let check_divided_tok4 = divide_tokens(check_tok4);

                let tok1_cmd4 =  Token::get_command("ls".to_string());
                let tok3_cmd4 = Token::get_command("ls".to_string());
                let mut divided_tok4_1_2 = Vec::new();
                divided_tok4_1_2.push(tok1_cmd4);
                let mut divided_tok4_2_2 = Vec::new();
                divided_tok4_2_2.push(tok3_cmd4);
                let mut divided_tok4 = Vec::new();
                divided_tok4.push(divided_tok4_1_2);
                divided_tok4.push(divided_tok4_2_2);


                assert_eq!(check_divided_tok4, divided_tok4);
            }

            #[test]
            fn check_getCommand_checkCommands_cmd5() {
                let cmd5 = "ls;;".to_string();

                let tok1_cmd5 = Token::get_command("ls".to_string());
                let tok2_cmd5 = Token::get_semi_colon();
                let tok3_cmd5 = Token::get_semi_colon();

                let mut tok5 = Vec::new();
                tok5.push(tok1_cmd5);
                tok5.push(tok2_cmd5);
                tok5.push(tok3_cmd5);

                assert_eq!(get_token(&cmd5).unwrap(), tok5);
                let mut check_tok5 = get_token(&cmd5).unwrap();
                assert_eq!(check_commands(check_tok5), Err(ShellError::CommandAfterOperator));

            }

        }
    }

    pub mod CommandExecuter {
        use std::io::{self, BufRead, Read, Stdin, Write};
        use crate::shell_error::*;
        use crate::shell_variables;
        use crate::token::*;
        use crate::input::*;
        use crate::output::*;
        use crate::instruction::*;
        use crate::command_handler::handler::*;
        use std::sync::Arc;
        use tokio::sync::Mutex;
        use std::str::FromStr;
        use crate::signal_handler::SignalHandler::handle_ctrl_c;
        use tokio::task;
        use tokio::io::{DuplexStream, duplex,AsyncRead, AsyncWrite, AsyncWriteExt};
        use std::process::Stdio;
        use crate::shell_variables::ShellVariables;
        use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
        #[derive(Clone, PartialEq)]
        pub enum IsSpawn {
            SPAWN,
            NOTSPAWN,
        }

        pub async fn execute(shell_variables: Arc<Mutex<ShellVariables>>, instructions_or_tokens: Vec<InstructionOrToken>) -> i32 {
            and_or(shell_variables, instructions_or_tokens).await
        }


        pub async fn and_or(shell_variables: Arc<Mutex<ShellVariables>>, instructions_or_tokens: Vec<InstructionOrToken>) -> i32{
            let mut instructions_or_tokens_to_execute: Vec<InstructionOrToken> = Vec::new();
            let mut status_result = 0;
            for instruction_or_token in instructions_or_tokens {
                if let InstructionOrToken::Instruction(_) = instruction_or_token {
                    instructions_or_tokens_to_execute.push(instruction_or_token);
                }
                else if let InstructionOrToken::Token(token) = instruction_or_token {
                    let shell_variables_clone = shell_variables.clone();
                    if Token::is_and(&token) {
                        status_result = background(shell_variables_clone, std::mem::take(&mut instructions_or_tokens_to_execute)).await;
                        if status_result != 0 {
                            return status_result;
                        }
                    } else if Token::is_or(&token) {
                        status_result = background(shell_variables_clone, std::mem::take(&mut instructions_or_tokens_to_execute)).await;
                        if status_result==0 {
                            return status_result;
                        }
                    } else {
                        instructions_or_tokens_to_execute.push(InstructionOrToken::Token(token));
                    }
                }
            }
            if !instructions_or_tokens_to_execute.is_empty() {
                return background(shell_variables, instructions_or_tokens_to_execute).await;
            }
            status_result
        }

        pub async fn background(shell_variables: Arc<Mutex<ShellVariables>>, instructions_or_tokens: Vec<InstructionOrToken>) -> i32 {
            let mut instructions_or_tokens_to_execute: Vec<InstructionOrToken> = Vec::new();
            for instruction_or_token in instructions_or_tokens {
                if let InstructionOrToken::Instruction(_) = instruction_or_token {
                    instructions_or_tokens_to_execute.push(instruction_or_token);
                } else if let InstructionOrToken::Token(token) = instruction_or_token {
                    if Token::is_background(&token) {
                        let instructions_tokens_in_background = std::mem::take(&mut instructions_or_tokens_to_execute);
                        let shell_variables_clone = shell_variables.clone();
                        task::spawn(async move {
                            pipe(shell_variables_clone, instructions_tokens_in_background, IsSpawn::SPAWN).await;
                        });
                    } else {
                        instructions_or_tokens_to_execute.push(InstructionOrToken::Token(token));
                    }
                }

            }
            if !instructions_or_tokens_to_execute.is_empty() {
                return pipe(shell_variables, instructions_or_tokens_to_execute, IsSpawn::NOTSPAWN).await;
            }
            0
        }

        // Ici il faut que j'ai des instructions qui s'enchainent
        pub async fn pipe(shell_variables: Arc<Mutex<ShellVariables>>, instructions_or_tokens: Vec<InstructionOrToken>, is_spawn: IsSpawn) -> i32 {
            let mut instructions_or_tokens_peekable = instructions_or_tokens.into_iter().peekable();
            let mut handles= Vec::new();
            let mut previous_pipe = false;
            while let Some(instruction_or_token) = instructions_or_tokens_peekable.next() {
                let mut instruction = match instruction_or_token {
                    InstructionOrToken::Instruction(instruction) => {instruction},
                    _ => panic!("CommandExecuter::pipe must have Instruction but have something else"),
                };

                if previous_pipe {
                    instruction.set_i(Input::Pipe);
                } 

                let is_pipe = match instructions_or_tokens_peekable.next() {
                    Some(InstructionOrToken::Token(token)) if Token::is_pipe(&token) => {previous_pipe=true;true},
                    Some(_) => panic!("CommandExecuter::pipe must have Token pipe after instruction but found something else"),
                    None => false,
                };


                if is_pipe {
                    instruction.set_o(Output::Pipe);
                }
                
                // Je crée mon child
                let mut shell_variables_clone = shell_variables.clone();
                let mut shell_variables_lock = shell_variables_clone.lock().await;
                let mut cmd = match shell_variables_lock.exec_instruction(&mut instruction, is_spawn.clone()).await {
                    Ok(cmd) => {drop(shell_variables_lock); cmd},
                    Err(status) => {handles.push(Err(status)); drop(shell_variables_lock); continue;}, // builtin ou erreur
                };
                
                match instruction.take_i_put_stdin() {
                    Input::Stdin => {
                        cmd.stdin(std::process::Stdio::inherit());
                    }
                    Input::File(path) => {
                        let file = std::fs::File::open(path).unwrap();
                        cmd.stdin(std::process::Stdio::from(file));
                    }
                    Input::Pipe => {
                        cmd.stdin(std::process::Stdio::piped());
                    }
                }

                match instruction.take_o_put_stdout() {
                    Output::Stdout => {
                        cmd.stdout(std::process::Stdio::inherit());
                    }
                    Output::FileOverwrite(path) => {
                        let file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(path).unwrap();
                        cmd.stdout(std::process::Stdio::from(file));
                    }
                    Output::FileAppend(path) => {
                        let file = std::fs::OpenOptions::new().write(true).create(true).append(true).open(path).unwrap();
                        cmd.stdout(std::process::Stdio::from(file));
                    }
                    Output::Pipe => {
                        cmd.stdout(std::process::Stdio::piped());
                    }
                }
                
                
                let mut handle = cmd.spawn().unwrap();
                handles.push(Ok(handle));
                
            }

            for i in 0..handles.len() - 1 {
                let stdout_prev = match &mut handles[i] {
                    Ok(child) => child.stdout.take(),
                    Err(_) => None,
                };
                let stdin_next = match &mut handles[i + 1] {
                    Ok(child) => child.stdin.take(),
                    Err(_) => None,
                };

                if let (Some(mut out), Some(mut inp)) = (stdout_prev, stdin_next) {
                    tokio::spawn(async move {
                        tokio::io::copy(&mut out, &mut inp).await.unwrap();
                    });
                }
            }

            if is_spawn == IsSpawn::NOTSPAWN {
                if let Some(last) = handles.last_mut() {
                    match last {
                        Ok(child) => {
                            return handle_ctrl_c(child).await;
                        }
                        Err(status) => {
                            return *status;
                        }
                    }
                } else {
                    return 0;
                }
            } else {
                0
            }
        }

        pub async fn exec_instruction(instruction: &mut Instruction, is_spawn: IsSpawn) -> tokio::process::Command {

            let cmd: String = instruction.get_command();
            let args: Vec<String> = instruction.get_args();

            let mut child_cmd = tokio::process::Command::new(cmd);
            child_cmd.args(args);

            return child_cmd;
        }
    }
}
