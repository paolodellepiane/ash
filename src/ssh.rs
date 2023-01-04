use crate::config::COMMON_SSH_ARGS;
use eyre::*;
use std::{
    io::{BufReader, Read, Write},
    process::{ChildStdin, ChildStdout, Command, Stdio},
};

pub struct Ssh {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Ssh {
    pub fn new(host_name: &str) -> Result<Self> {
        let mut child = Command::new("ssh")
            .args(COMMON_SSH_ARGS)
            .args(["-T", host_name])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| eyre!("can't take stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| eyre!("can't take stdout"))?;
        let mut stdout = BufReader::new(stdout);
        let mut buf = [0; 4096];
        _ = stdout.read(&mut buf)?;
        Ok(Self { stdin, stdout })
    }

    pub fn write(&mut self, cmd: &str) -> Result<()> {
        writeln!(self.stdin, "{cmd}")?;
        Ok(())
    }

    pub fn read(&mut self) -> Result<String> {
        let mut buf = [0; 4096];
        _ = self.stdout.read(&mut buf)?;
        let out = String::from_utf8_lossy(&buf);
        let out = out.trim_end_matches(char::from(0)).trim().to_string();
        Ok(out)
    }
}

impl Drop for Ssh {
    fn drop(&mut self) {
        match self.write("exit") {
            Ok(_) => {}
            Err(err) => println!("error closing ssh connection: {err:?}"),
        };
    }
}
