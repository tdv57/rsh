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
                    tokens.push(Token::get_command(command.clone()));
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
                        tokens.push(Token::get_command(command.clone()));
                        command.clear();
                    }

                    while let Some(&c) = chars.peek() {
                        if is_command(c) {
                            var.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    if var.is_empty() {
                        tokens.push(Token::get_command("$".to_string()));
                    } else {
                        tokens.push(Token::get_variable(var.clone()));
                        var.clear();
                    }
                } else if is_quote(char) {
                    if !command.trim().is_empty() {
                        tokens.push(Token::get_command(command.clone()));
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

                    tokens.push(Token::get_inquote(inquote.clone()));
                    inquote.clear();

                }  else {
                    command.push(char);
                }
            }
            // Erreur je peux avoir un opérateur à la fin
            // if let Some(Token::Operator(_)) = tokens.last() {
            //     return Err(ShellError::CommandAfterOperator);
            // }
            if !command.trim().is_empty() {
                tokens.push(Token::get_command(command.clone()));
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

        // A changer plus tard
        pub fn expand_variables(tokens: Vec<Token>) -> Result<Vec<Token>,ShellError> {
            Ok(tokens.into_iter().map(|token| {
                match token {
                    Token::NotOperator(TokenNotOperator::Variable(var_name)) => {
                        // Résoudre la variable pour la remplacer
                        Token::NotOperator(TokenNotOperator::Command(var_name.clone()))
                    },
                    Token::NotOperator(TokenNotOperator::Inquote(in_quote)) => {
                        // On pourra faire des plus compliqués ensuite en fonction de si on a "" ou '' au début
                        Token::NotOperator(TokenNotOperator::Inquote(in_quote))
                    },
                    _ => token,
                }
            }).collect())
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

                let tok1_cmd2 = Token::get_command("Bonjour ".to_string());
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

                let tok1_cmd3 = Token::get_command("echo bonjour a tous ".to_string());
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
                let tok3_cmd4 = Token::get_command(" ls".to_string());


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
                let tok3_cmd4 = Token::get_command(" ls".to_string());
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

        use tokio::task;
        use tokio::io::{DuplexStream, duplex,AsyncRead, AsyncWrite, AsyncWriteExt};
        use std::process::Stdio;
        use crate::shell_variables::ShellVariables;
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
                            pipe(shell_variables_clone, instructions_tokens_in_background).await;
                        });
                    } else {
                        instructions_or_tokens_to_execute.push(InstructionOrToken::Token(token));
                    }
                }

            }
            if !instructions_or_tokens_to_execute.is_empty() {
                return pipe(shell_variables, instructions_or_tokens_to_execute).await;
            }
            0
        }

        // Ici il faut que j'ai des instructions qui s'enchainent
        pub async fn pipe(shell_variables: Arc<Mutex<ShellVariables>>, instructions_or_tokens: Vec<InstructionOrToken>) -> i32 {
            let mut instructions_or_tokens_peekable = instructions_or_tokens.into_iter().peekable();
            let mut last_input_pipe: Option<DuplexStream> = None;
            let mut last_handle = None;

            while let Some(instruction_or_token) = instructions_or_tokens_peekable.next() {
                let mut instruction = match instruction_or_token {
                    InstructionOrToken::Instruction(instruction) => {instruction},
                    _ => panic!("CommandExecuter::pipe must have Instruction but have something else"),
                };

                let is_pipe = match instructions_or_tokens_peekable.next() {
                    Some(InstructionOrToken::Token(token)) if Token::is_pipe(&token) => true,
                    Some(_) => panic!("CommandExecuter::pipe must have Token pipe after instruction but found something else"),
                    None => false,
                };

                if let Some(input_pipe) = last_input_pipe.take() {
                    instruction.set_i(Input::Pipe(Box::new(input_pipe)));
                }

                let next_output_pipe = if is_pipe {
                    let (output_pipe, input_pipe) = duplex(1024);
                    last_input_pipe = Some(input_pipe);
                    Some(output_pipe)
                } else {
                    last_input_pipe = None;
                    None
                };

                if let Some(output_pipe) = next_output_pipe {
                    instruction.set_o(Output::Pipe(Box::new(output_pipe)));
                }
                let mut shell_variables_clone = shell_variables.clone();
                let handle = tokio::spawn(async move {
                    let mut shell_variables_lock = shell_variables_clone.lock().await;
                    shell_variables_lock.exec_instruction(instruction).await
                });

                last_handle = Some(handle);
            }

            if let Some(handle) = last_handle {
                match handle.await {
                    Ok(status) => return status,
                    Err(_) => return 1,
                }
            }
            0
        }

        pub async fn exec_instruction(mut instruction: Instruction) -> i32 {

            let input = instruction.take_i_put_stdin();
            let output = instruction.take_o_put_stdout();
            let cmd: String = instruction.get_command();
            let args: Vec<String> = instruction.get_args();

            let mut child_cmd = tokio::process::Command::new(cmd);
            child_cmd.args(args);

            match input {
                Input::Stdin => {
                    child_cmd.stdin(std::process::Stdio::inherit());
                }
                Input::File(path) => {
                    let file = std::fs::File::open(path).unwrap();
                    child_cmd.stdin(std::process::Stdio::from(file));
                }
                Input::Pipe(_) => {
                    child_cmd.stdin(std::process::Stdio::piped());
                }
            }

            match output {
                Output::Stdout => {
                    child_cmd.stdout(std::process::Stdio::inherit());
                }
                Output::FileOverwrite(path) => {
                    let file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(path).unwrap();
                    child_cmd.stdout(std::process::Stdio::from(file));
                }
                Output::FileAppend(path) => {
                    let file = std::fs::OpenOptions::new().write(true).create(true).append(true).open(path).unwrap();
                    child_cmd.stdout(std::process::Stdio::from(file));
                }
                Output::Pipe(_) => {
                    child_cmd.stdout(std::process::Stdio::piped());
                }
            }

            let mut child = child_cmd.spawn().unwrap();
            let status = child.wait().await.unwrap();
            status.code().unwrap_or(1)
        }
    }
}
