use anyhow::bail;
use core::panic;
use scanpw::scanpw;
use std::io::{Error, ErrorKind, Write};
use std::process::{Command, Stdio};
use std::{fs, path::PathBuf};

pub fn set_passphrase(passfile: &PathBuf) -> anyhow::Result<String> {
    let (mut pass1, mut pass2);
    loop {
        pass1 = scanpw!("Password: ");
        println!();
        pass2 = scanpw!("Confirm password: ");
        println!();
        if pass1 == pass2 {
            break;
        } else {
            println!("Passwords do not match! Try again.");
        }
    }
    encrypt(pass1.trim(), pass1.trim().as_bytes(), passfile)?;
    decrypt(passfile, None)
}

pub fn encrypt(passphrase: &str, data: &[u8], file: &PathBuf) -> anyhow::Result<()> {
    let mut child = Command::new("gpg")
        .arg("--batch")
        .arg("--passphrase")
        .arg(passphrase)
        .arg("--symmetric")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let data_clone = Vec::from(data);
    std::thread::spawn(move || {
        stdin
            .write_all(&data_clone)
            .expect("Failed to write to stdin");
    });
    let output = child.wait_with_output()?;
    if !output.status.success() {
        panic!("{0}", String::from_utf8(output.stderr)?);
    }
    let parent = file.parent().unwrap();
    if !parent.exists() {
        fs::create_dir_all(parent)?
    }
    Ok(fs::write(file, output.stdout)?)
}

pub fn decrypt(file: &PathBuf, passphrase: Option<&str>) -> anyhow::Result<String> {
    if !file.exists() {
        bail!(Error::new(ErrorKind::NotFound, format!("file {file:#?} not found")))
    }
    let output = if let Some(passphrase) = passphrase {
        Command::new("gpg")
            .arg("--batch")
            .arg("--passphrase")
            .arg(passphrase)
            .arg("--decrypt")
            .arg(file)
            .output()?
    } else {
        Command::new("gpg")
            .arg("--decrypt")
            .arg(file)
            .output()?
    };
    if !output.status.success() {
        panic!("{0}", String::from_utf8(output.stderr)?);
    }
    Ok(String::from_utf8(output.stdout)?)
}
