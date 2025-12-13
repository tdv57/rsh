use std::io::{self, BufRead, Read, Write};
use std::fmt::Display;
use std::error::Error;


#[derive(Debug, Clone, PartialEq)]
pub enum ShellError {
    QuoteNotClosed(String),
    ErrorWhileRestoringString,
    CommandError(String),
    EmptyCommand,
    UnknownError(String),
    CommandFirst,
    CommandAfterOperator,
    EmptyVar,
}

impl Error for ShellError {}

impl Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::QuoteNotClosed(s) =>  format!("quote unclosed detected :: {}", s),
            Self::ErrorWhileRestoringString => String::from("unexpected error while restoring strings"),
            Self::CommandError(s) => format!("command error :: {}", s),
            Self::EmptyCommand => format!("found an empty command"),
            Self::UnknownError(s) => format!("Unknown error append in :: {}", s),
            Self::CommandFirst => format!("instruction has started with an operator and not a command"),
            Self::CommandAfterOperator => format!("operator can't be followed by an operator"),
            Self::EmptyVar => format!("encountered an empty var"),
        };
        write!(f, "Error: {message}")
    }
}

impl ShellError {
    pub fn handle_shell_error<T>(result: Result<T, Self>) -> Result<T, i32> {
        match result {
            Ok(res) => {return Ok(res);},
            Err(error ) => {
                println!("{}", error);
                return match error {
                    Self::QuoteNotClosed(_) => Err(1),
                    Self::ErrorWhileRestoringString => Err(2),
                    Self::CommandError(_) => Err(3),
                    Self::EmptyCommand => Err(4),
                    Self::CommandFirst => Err(5),
                    Self::CommandAfterOperator => Err(6),
                    Self::EmptyVar => Err(7),
                    Self::UnknownError(_) => Err(255),
                    _ => {panic!("Une ShellError n'est pas gérée:: todo()!");},
                };
            }
        }

    }
}


