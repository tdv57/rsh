use std::fmt::Display;
use std::error::Error;
use std::process::Output;
use tokio::io::{AsyncReadExt, DuplexStream};
use std::io::{self, Read};
use std::fs::File;

use crate::instruction::Instruction;

#[derive(Debug)]
pub enum Input {
    Stdin,
    File(String),
    Pipe(Box<DuplexStream>),
}

impl Input {
    fn read_from_file(file: String) -> io::Result<String> {
        let mut buffer = String::new();
        let mut file_open = File::open(file)?;
        file_open.read_to_string(&mut buffer)?;
        Ok(buffer)
    }

    async fn read_from_pipe(mut pipe: Box<DuplexStream>) -> String {
        let mut buffer: String = String::new();
        pipe.read_to_string(&mut buffer).await;
        buffer
    }

    fn read_from_stdout(args: Vec<String>) -> String {
        args.join(" ")
    }

    pub async fn read(mut instruction: Instruction) -> String {
        match instruction.take_i_put_stdin() {
            Input::File(file) => Self::read_from_file(file).expect("Output::read::read_from_file erreur lors de l'ouverture du fichier"),
            Input::Stdin => Self::read_from_stdout(instruction.get_args()),
            Input::Pipe(pipe) => Self::read_from_pipe(pipe).await,
        }
    }
}