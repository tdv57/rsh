use std::fmt::Display;
use std::error::Error;
use std::io::{self, Write};
use tokio::io::{ AsyncReadExt, AsyncWrite, AsyncWriteExt, DuplexStream};
use std::fs::File;
use crate::instruction::Instruction;
use std::fs::OpenOptions;
use crate::instruction;

#[derive(Debug)]
pub enum Output{
    Stdout,
    FileAppend(String),
    FileOverwrite(String),
    Pipe(Box<DuplexStream>),
}

impl Output {
    fn write_to_file_overwrite(file: String, buffer: String) -> io::Result<()> {
        let mut file_open = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(file)?;
        file_open.write_all(buffer.as_bytes())?;
        Ok(())
    }

    fn write_to_file_append(file: String, buffer: String) -> io::Result<()> {
        let mut file_open = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(file)?;
            file_open.write_all(buffer.as_bytes())?;
            Ok(())
    }

    async fn write_to_pipe(mut pipe: Box<DuplexStream>, buffer: String) -> () {
        pipe.write_all(buffer.as_bytes()).await;
        pipe.flush().await;

    }

    fn write_to_stdout(buffer: String) -> () {
        print!("{}",buffer);
    }

    pub async fn write(mut instruction: Instruction, buffer: String) -> () {
        match instruction.take_o_put_stdout() {
            Output::FileAppend(file) => Self::write_to_file_append(file, buffer).expect("Output::write::write_to_file_append erreur lors de l'ouverture du ficher"),
            Output::FileOverwrite(file) => Self::write_to_file_overwrite(file, buffer).expect("Output::write::write_to_file_overwrite erreur lors de l'ouverture du ficher"),
            Output::Pipe(pipe) => Self::write_to_pipe(pipe, buffer).await,
            Output::Stdout => Self::write_to_stdout(buffer),
        };
    }
}