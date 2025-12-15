
/* SHELL SIMPLIFIE REGLES:

Liste des opérateurs : > < >> || && & | $VAR COMMAND

Règle 1: On commence toujours par une Command ShellError::CommandFirst
Règle 2: Un opérateur est toujours suivit d'une commande

Ordre de traitement

1) () --> pour le moment on oublie
2) $ + Restorations des String
3) < > >>
4) && et || 
5) &
6) | 
*/
use std::fmt::{self, Display};
use std::error::Error;
use crate::shell_error::ShellError;
use crate::input::*;
use crate::output::*;
use std::io;
use crate::instruction::*;

use std::fs;
use std::fs::OpenOptions;

#[derive(Debug, Clone)]
pub enum TokenError {
    OnlyNotOperatorToken(TokenOperator),
    NoToken,
}

impl Error for TokenError {}

impl Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::OnlyNotOperatorToken(_) => format!("Should have only TokenNotOperator"),
            Self::NoToken => format!("No token found in Vec<Token>"),
        };
        write!(f, "Error: {message}")
    }
}



#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    Operator(TokenOperator),
    NotOperator(TokenNotOperator),
}

impl Token {
    pub fn get_command(command: String) -> Token {
        Token::NotOperator(TokenNotOperator::Command(command))
    }

    pub fn get_variable(var: String) -> Token {
        Token::NotOperator(TokenNotOperator::Variable(var))
    }

    pub fn get_inquote(inquote: String) -> Token {
        Token::NotOperator(TokenNotOperator::Inquote(inquote))
    }

    pub fn get_redirection_input() -> Token {
        Token::Operator(TokenOperator::Redirection(TokenRedirection::RedirectionInput))
    }

    pub fn get_redirection_output_append() -> Token {
        Token::Operator(TokenOperator::Redirection(TokenRedirection::RedirectionOutputAppend))
    }

    pub fn get_redirection_output_overwrite() -> Token {
        Token::Operator(TokenOperator::Redirection(TokenRedirection::RedirectionOutputOverwrite))
    }

    pub fn get_left_paren() -> Token {
        Token::Operator(TokenOperator::Logic(TokenLogic::LeftParen))
    }

    pub fn get_right_paren() -> Token {
        Token::Operator(TokenOperator::Logic(TokenLogic::RightParen))
    }

    pub fn get_background() -> Token {
        Token::Operator(TokenOperator::Logic(TokenLogic::Background))
    }

    pub fn get_and() -> Token {
        Token::Operator(TokenOperator::Logic(TokenLogic::And))
    }    

    pub fn get_or() -> Token {
        Token::Operator(TokenOperator::Logic(TokenLogic::Or))
    }    

    pub fn get_pipe() -> Token {
        Token::Operator(TokenOperator::Logic(TokenLogic::Pipe))
    }    

    pub fn get_semi_colon() -> Token {
        Token::Operator(TokenOperator::Logic(TokenLogic::SemiColon))
    }   

    pub fn is_and(&self) -> bool {
        matches!(
            self, 
            Token::Operator(TokenOperator::Logic(TokenLogic::And))
        )
    }

    pub fn is_or(&self) -> bool {
        matches!(
            self, 
            Token::Operator(TokenOperator::Logic(TokenLogic::Or))
        )
    }

    pub fn is_background(&self) -> bool {
        matches!(self, Token::Operator(TokenOperator::Logic(TokenLogic::Background)))
    }

    pub fn is_pipe(&self) -> bool {
        matches!(self, Token::Operator(TokenOperator::Logic(TokenLogic::Pipe)))
    }

    pub fn is_semi_colon(&self) -> bool {
        matches!(self, Token::Operator(TokenOperator::Logic(TokenLogic::SemiColon)))
    }

    pub fn is_var(&self) -> bool {
        matches!(self, Token::NotOperator(TokenNotOperator::Variable(_)))
    }

    pub fn is_command(&self) -> bool {
        matches!(self, Token::NotOperator(TokenNotOperator::Command(_)))
    }

    pub fn is_redirection(&self) -> bool {
        matches!(self, Token::Operator(TokenOperator::Redirection(_)))
    }

    pub fn is_logic(&self) -> bool {
        matches!(self, Token::Operator(TokenOperator::Logic(_)))
    }
    
    pub fn is_inquote(&self) -> bool {
        matches!(self, Token::NotOperator(TokenNotOperator::Inquote(_)))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenNotOperator {
    Command(String),
    Variable(String), // $___
    Inquote(String),
}

impl TokenNotOperator {
    pub fn trim_inquote(inquote: String) -> String {
        let first_char = match inquote.chars().next() {
            None => return inquote,
            Some(c) => c,
        };
        let last_char = match inquote.chars().rev().next() {
            None => return inquote,
            Some(c) => c,
        };

        if (first_char == '"' && last_char == '"') || (first_char == '\'' && last_char == '\'') {
            return inquote[1..inquote.len()-1].to_string();
        }
        return inquote;
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenOperator {
    Redirection(TokenRedirection),
    Logic(TokenLogic),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenRedirection {
    RedirectionInput, // <
    RedirectionOutputAppend, // >>
    RedirectionOutputOverwrite, // >
}

impl TokenRedirection {
    // Je dois avoir une fonction qui prend une Instruction, un token Redirection et un Token Command ou inquote et qui traite la redirection
    pub fn apply_redirection(instruction: &mut Instruction, token_redirection: Self, cmd_or_inquote: Token) -> io::Result<()> {
        
        let mut file = String::new();
        let mut args: Vec<String> = Vec::new();
        
        match cmd_or_inquote {
            Token::NotOperator(TokenNotOperator::Command(cmd)) => {
                args.extend(cmd.split_whitespace().map(|s| s.to_string()));
                file = args.remove(0);
                instruction.add_args(args.clone());
            },
            Token::NotOperator(TokenNotOperator::Inquote(inquote)) => {
                file = inquote;
            },
            _ => panic!("TokenRedirection::apply_redirection -> cmd_or_inquote must be command or inquote"),
        }
        match token_redirection {
            TokenRedirection::RedirectionInput => {
                //
                instruction.set_i(Input::File(file.clone()));
                match fs::metadata(&file) {
                    Ok(metadata) => {
                        instruction.add_args(args);
                        instruction.set_i(Input::File(file));
                    },
                    Err(err) => {
                        return Err(err);
                    },
                }
                
            },
            TokenRedirection::RedirectionOutputAppend => {
                instruction.set_o(Output::FileAppend(file.clone()));
                OpenOptions::new()
                    .write(true)
                    .truncate(false)
                    .create(true)  
                    .open(&file)?;
            }, 
            TokenRedirection::RedirectionOutputOverwrite => {
                instruction.set_o(Output::FileOverwrite(file.clone()));
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)   
                    .open(&file)?;
            },
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenLogic {
    LeftParen, // '('
    RightParen, //')'
    Background, // &
    And, // &&
    Or, // || 
    Pipe, // | 
    SemiColon, // ; 
}


pub fn is_operator(c: char) -> bool {
    ['>', '<', '|', '&', ';'].contains(&c)
}

pub fn is_var(c: char) -> bool {
    c=='$'
}

pub fn is_command(c: char) -> bool {
     !is_operator(c) && !is_var(c)
}

pub fn is_quote(c: char) -> bool {
    ['\'', '\"'].contains(&c)
}





impl Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Operator(op) => match op {
                TokenOperator::Logic(logic) => match logic {
                    TokenLogic::And => write!(f, "&&")?,
                    TokenLogic::Background => write!(f, "&")?,
                    TokenLogic::LeftParen => write!(f, "(")?,
                    TokenLogic::Or => write!(f, "||")?,
                    TokenLogic::Pipe => write!(f, "|")?,
                    TokenLogic::RightParen => write!(f, ")")?,
                    TokenLogic::SemiColon => write!(f, ";")?,
                },
                TokenOperator::Redirection(redir) => match redir {
                    TokenRedirection::RedirectionInput => write!(f, "<")?,
                    TokenRedirection::RedirectionOutputAppend => write!(f, ">>")?,
                    TokenRedirection::RedirectionOutputOverwrite => write!(f, ">")?,
                },
            },
            Token::NotOperator(notop) => match notop {
                TokenNotOperator::Command(cmd) => write!(f, "Command({})", cmd)?,
                TokenNotOperator::Variable(var) => write!(f, "Variable({})", var)?,
                TokenNotOperator::Inquote(val) => write!(f, "Inquote({})", val)?,
            },
        }
        Ok(())
    }
}