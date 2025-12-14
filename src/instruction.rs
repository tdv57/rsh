
use crate::output::*;
use crate::input::*;
use crate::token::*;

#[derive(Debug)]
pub struct Instruction {
    command: String,
    args: Vec<String>,
    input: Input,
    output: Output,
}


impl Instruction { 
    pub fn new() -> Self {
        Self {command: String::new(), 
              args: Vec::new(), 
              input: Input::Stdin, 
              output: Output::Stdout}
    }

    pub fn set_command(&mut self, command: String)  -> Option<()>{
        if command.is_empty() {
            return None;
        }
        self.command = command;
        Some(())
    }

    pub fn get_command(&self) -> String {
        self.command.clone()
    }

    pub fn add_args(&mut self, args: Vec<String>) {
        self.args.extend(args);
    }

    pub fn get_args(&self) -> Vec<String> {
        self.args.clone()
    }

    pub fn get_len_args(&self) -> usize {
        self.args.len()
    }

    pub fn from(command: String, args: Vec<String>, input: Input, output: Output) -> Option<Self> {
        if command.is_empty() {
            return None;
        }
        Some(Self {command, args, input, output})
    }

    pub fn add_cmd_or_inquote_token(&mut self, token: Token) -> () {


        if self.command.is_empty() && self.args.len() != 0 {
            panic!("Instruction::add_cmd_or_inquote_token command empty in Instruction but not args");
        }

        let mut command = String::new();
        let mut args: Vec<String> = Vec::new();

        match token {
            Token::NotOperator(TokenNotOperator::Command(cmd)) => {
                if self.command.is_empty() {
                    self.args.extend(cmd.split_whitespace().map(|s| s.to_string()));
                    self.command = self.args.remove(0);
                } else {
                    self.args.extend(cmd.split_whitespace().map(|s| s.to_string()));
                }

            },
            Token::NotOperator(TokenNotOperator::Inquote(inquote)) => {
                let trim_inquote = TokenNotOperator::trim_inquote(inquote);
                if self.command.is_empty() {
                    self.command = trim_inquote;
                } else {
                    self.args.push(trim_inquote);
                }
            },
            _ => panic!("Instruction::add_cmd_or_inquote_token must have command or inquote token"),
        }
    }

    pub fn clear(&mut self) -> () {
        self.command = String::new();
        self.args = Vec::new();
        self.input = Input::Stdin;
        self.output = Output::Stdout;
    }

    pub fn is_empty(&mut self) -> bool {
        if (self.command.is_empty()) {
            if (self.args.len() != 0) {
                panic!("Instruction::is_empty empty command but not args in Instruction");
            }
            return true;
        }
        false
    }

    pub fn from_token(mut tokens: &[Token]) -> Result<Self, TokenError> {
        if tokens.is_empty() {
            return Err(TokenError::NoToken);
        }
        let mut command = String::new();
        let mut args: Vec<String> = Vec::new();
        let (first_token, tokens) = tokens.split_first().ok_or(TokenError::NoToken)?;
        
        match first_token {
            Token::NotOperator(TokenNotOperator::Command(cmd)) => {
                args.extend(cmd.split_whitespace().map(|s| s.to_string()));
                command = args.remove(0);
            }, 
            Token::NotOperator(TokenNotOperator::Inquote(inquote)) => {
                command = inquote.clone();
            },
            Token::Operator(first_token_operator) =>  return Err(TokenError::OnlyNotOperatorToken(first_token_operator.clone())),
            _ => {},
        }
        for token in tokens {
            match token {
                Token::NotOperator(TokenNotOperator::Command(cmd)) => {
                    args.extend(cmd.split_whitespace().map(|s| s.to_string()));
                }, 
                Token::NotOperator(TokenNotOperator::Inquote(inquote)) => {
                    args.push(inquote.clone());
                },
                _ => {
                    if let Token::Operator(first_token_operator) = token {
                        return Err(TokenError::OnlyNotOperatorToken(first_token_operator.clone()));
                    };
                },
            }
        }
        Ok(Self {
            command,
            args,
            input: Input::Stdin,
            output: Output::Stdout,
        })
    }

    pub fn set_io(&mut self, input: Input, output: Output) {
        self.input = input;
        self.output = output;
    }

    pub fn set_i(&mut self, input: Input) {
        self.input = input;
    }

    pub fn get_i(&self) -> &Input {
        &self.input 
    }

    pub fn take_i_put_stdin(&mut self) -> Input {
        std::mem::replace(&mut self.input, Input::Stdin)
    }

    pub fn set_o(&mut self, output: Output) {
        self.output = output;
    }

    pub fn get_o(&self) -> &Output {
        &self.output
    }

    pub fn take_o_put_stdout(&mut self) -> Output {
        std::mem::replace(&mut self.output, Output::Stdout)
    }



}

type Instructions = Vec<Instruction>;