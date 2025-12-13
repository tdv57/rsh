// use std::collections::HashMap;
// use std::io::{self, BufRead, Read, Write};
// use crate::shell_error::*;
// use crate::token::*;
// use crate::input::*;
// use crate::output::*;
// use crate::command_executer::*;
// use crate::instruction::*;
// use std::str::FromStr;

// pub struct CommandParser{
//     // Devra gérer les ">" , les ";" les "|" les " " en trop
//     // attention si il ya "" ou ' il faut qu'il soit fermé
//     // Je vais prendre une structure qui va remplacer toutes les " ... " et '...' par "i" où i est l'index de variable "" ou '' rencontré et stoquer les string dans un vec de string
//     // se sera fait par le string manager
//     // renverra un vec de String ou chaque string est une commande shell
// }

// impl FromStr for CommandParser {
//     type Err = ShellError;

//     fn from_str(command: &str) -> Result<Self, Self::Err> {

//         let mut tokens = Self::get_token(command)?;
//         tokens = Self::check_commands(tokens)?;
//         let divided_tokens = Self::divide_tokens(tokens);
//         let instructions_or_tokens: Vec<InstructionOrToken> = Vec::new();
//         for divided_token in divided_tokens {
//             let expanded_token = Self::expand_variables(divided_token)?;
//             Self::build_instructions(expanded_token);
//             // Je dois directement exécuter
//         }
//         Ok(Self {instructions_or_tokens})
        
//     }
// }

// impl CommandParser {
//     fn get_token(command: &str) -> Result<Vec<Token>, ShellError> {
//         let mut chars = command.chars().peekable();
//         let mut all_tokens: Vec<Vec<Token>> = Vec::new(); 
//         let mut tokens: Vec<Token> = Vec::new();
//         //1 doit commencer par une commande
//         let mut command = String::new();
//         let mut var = String::new();
//         let mut inquote = String::new();

//         if let Some(&first_char) = chars.peek() {
//             if (is_operator(first_char)) {
//                 return Err(ShellError::CommandFirst);
//             }
            
//         }

//         while let Some(char) = chars.next() {
//             if is_operator(char) {
//                 if command.is_empty() {
//                     return Err(ShellError::CommandAfterOperator);
//                 }
//                 tokens.push(Token::get_command(command.clone()));
//                 command.clear();

//                 match char {
//                     '>' => {
//                             if chars.peek()==Some(&'>') {
//                                 chars.next();
//                                 tokens.push(Token::get_redirection_output_append());
//                             } else {
//                                 tokens.push(Token::get_redirection_output_overwrite());
//                             }
//                     } ,

//                     '<' => tokens.push(Token::get_redirection_input()),
            
//                     '|' => {
//                         if chars.peek() == Some(&'|') {
//                             chars.next();
//                             tokens.push(Token::get_or())
//                         } else {
//                             tokens.push(Token::get_pipe())
//                         }
//                     },

//                     '&' => {
//                         if chars.peek() == Some(&'&') {
//                             chars.next();
//                             tokens.push(Token::get_and())
//                         } else {
//                             tokens.push(Token::get_background())
//                         }
//                     }, 

//                     ';' => tokens.push(Token::get_semi_colon()),
//                 _ => {},
//                 }
//             } 
//             else if is_var(char) {
//                 if !command.is_empty() {
//                     tokens.push(Token::get_command(command.clone()));
//                     command.clear();
//                 }

//                 while let Some(&c) = chars.peek() {
//                     if is_command(c) {
//                         var.push(c);
//                         chars.next();
//                     } else {
//                         break;
//                     }
//                 }

//                 if var.is_empty() {
//                     return Err(ShellError::EmptyVar);
//                 } else {
//                     tokens.push(Token::get_variable(var.clone()));
//                     var.clear();
//                 }
//             } else if is_quote(char) {
//                 if !command.is_empty() {
//                     tokens.push(Token::get_command(command.clone()));
//                     command.clear();
//                 }

//                 while let Some(&c) = chars.peek() {
//                     if c == char {
//                         break;
//                     } else {
//                         inquote.push(c);
//                         chars.next();
//                     }
//                 }

//                 if chars.peek() == None {
//                     return Err(ShellError::QuoteNotClosed(inquote.clone()));
//                 }
                
//                 tokens.push(Token::get_inquote(inquote.clone()));
//                 inquote.clear();

//             }  else {
//                 command.push(char);
//             }
//         }
//         // Erreur je peux avoir un opérateur à la fin
//         // if let Some(Token::Operator(_)) = tokens.last() {
//         //     return Err(ShellError::CommandAfterOperator);
//         // }
//         Ok(tokens)
//     }

//     // Doit checker que la première commande est soit > >> ou une commande classique
//     fn check_commands(tokens: Vec<Token>) -> Result<Vec<Token>, ShellError> {
//         let mut peekable_tokens = tokens.iter().peekable();
//         while let Some(token) = peekable_tokens.next() {
//             if let Token::Operator(_) = token {
//                 if let Some(next_token) = peekable_tokens.peek() {
//                     if let Token::Operator(_) = **next_token {
//                         return Err(ShellError::CommandAfterOperator);
//                     }
//                 }
//             }
//         }
//         Ok(tokens)
//     }

//     fn divide_tokens(tokens: Vec<Token>) -> Vec<Vec<Token>> {
//         let mut tokens_divided: Vec<Vec<Token>> = Vec::new();
//         let mut tokens_to_execute: Vec<Token> = Vec::new();
//         for token in tokens {
//             if Token::is_semi_colon(&token) {
//                 tokens_divided.push(tokens_to_execute.clone());
//                 tokens_to_execute.clear();
//             } else {
//                 tokens_to_execute.push(token);
//             }
//         }
//         if (!tokens_to_execute.is_empty()) {
//             tokens_divided.push(tokens_to_execute);
//         }
//         tokens_divided
//     }

//     // A changer plus tard 
//     fn expand_variables(tokens: Vec<Token>) -> Result<Vec<Token>,ShellError> {
//         Ok(tokens.into_iter().map(|token| {
//             match token {
//                 Token::NotOperator(TokenNotOperator::Variable(var_name)) => {
//                     // Résoudre la variable pour la remplacer
//                     Token::NotOperator(TokenNotOperator::Command(var_name.clone()))
//                 },
//                 Token::NotOperator(TokenNotOperator::Inquote(in_quote)) => {
//                     // On pourra faire des plus compliqués ensuite en fonction de si on a "" ou '' au début
//                     Token::NotOperator(TokenNotOperator::Inquote(in_quote))
//                 },
//                 _ => token,
//             }
//         }).collect())
//     }


//     fn build_instructions(tokens: Vec<Token>) -> Vec<InstructionOrToken> {
//         // Je dois parcourir les tokens soit c'est une commande et donc je dois merge 
//         // Soit c'est > >> <  et donc fin de commande + modification de fichier voir commandes derrière
//         // Soit c'est | donc j'écrase stdout et je passe à construire la commande suivante ou push le token
//         // Soit c'est un opérateur donc je stoppe la commande je la push et je push le Token

//         let mut instructions_or_tokens: Vec<InstructionOrToken> = Vec::new();
//         let mut instruction = Instruction::new();

//         let mut tokens_iter = tokens.into_iter().peekable();

//         while let Some(token) = tokens_iter.next() {
//             if Token::is_command(&token) || Token::is_inquote(&token) {
//                 instruction.add_cmd_or_inquote_token(token);
//             } else if let Token::Operator(TokenOperator::Redirection(redirection_token)) = token  {
//                 if let Some(next_token) = tokens_iter.next() {
//                     TokenRedirection::apply_redirection(&mut instruction, redirection_token, next_token);
//                 }
//             } else if Token::is_logic(&token) {
//                 instructions_or_tokens.push(InstructionOrToken::Instruction(instruction));
//                 instruction = Instruction::new();
//                 instructions_or_tokens.push(InstructionOrToken::Token(token));
//                 // Je stoppe la formation de l'instruction je push l'instruction et je push le token
//             } else {
//                 panic!("Token must be Logic Redirection, Command or Inquote");
//             }
//         }
//         if !instruction.is_empty() {
//             instructions_or_tokens.push(InstructionOrToken::Instruction(instruction));
//         }
//         instructions_or_tokens
//     }
// }



// // Data structur --> 
// /* Command {
//     args: String
    
//     } */
