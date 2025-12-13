use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use std::io::{Write};

pub struct IOManager {
    pub stdout : BufWriter<tokio::io::Stdout>,
    pub stdin : BufReader<tokio::io::Stdin>,
    pub buffer : String,
}

impl IOManager {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            stdin: BufReader::new(io::stdin()),
            stdout: BufWriter::new(io::stdout()),
        }

    }

    pub async fn clear_all(&mut self) -> io::Result<()> {
        self.stdout.flush().await?;
        self.buffer.clear();
        Ok(())
    }

    pub async fn read_line(&mut self) -> io::Result<usize> {
        let n_line = self.stdin.read_line(&mut self.buffer).await?;
        Ok(n_line)
    }

    pub async fn get_line(&self) -> String {
        self.buffer.clone()
    }

    pub async fn write_all(&mut self, to_print: &str) -> io::Result<()>{
        self.stdout.write_all(to_print.as_bytes()).await?;
        self.stdout.flush().await?;
        Ok(())
    }
}

pub fn read_line_sync(prompt: &str) -> String {
    print!("{}", prompt);
    std::io::stdout().flush().unwrap();
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer).unwrap();
    buffer.trim_end().to_string()
}