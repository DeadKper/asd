use anyhow::{anyhow, bail, Ok};
use log::{debug, warn};
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
            warn!("Passwords do not match!");
        }
    }
    encrypt(pass1.trim(), pass1.trim().as_bytes(), passfile)?;
    Ok(pass1.trim().to_owned())
}

pub fn get_passphrase(passfile: &PathBuf) -> anyhow::Result<String> {
    if passfile.exists() {
        let output = Command::new("gpg")
            .arg("--decrypt")
            .arg(passfile)
            .output()?;
        if !output.status.success() {
            bail!(anyhow!(
                "{}",
                String::from_utf8(output.stderr)?
                    .trim()
                    .rsplit_once("\n")
                    .unwrap()
                    .1
            ));
        }
        return Ok(String::from_utf8(output.stdout)?.trim().to_owned());
    } else {
        warn!("passfile {passfile:?} not found");
        warn!("did you forget to 'asd config init' or 'asd config passphrase'?")
    }
    let pass = scanpw!("Password: ");
    println!();
    Ok(pass)
}

pub fn encrypt(passphrase: &str, data: &[u8], file: &PathBuf) -> anyhow::Result<()> {
    let mut child = Command::new("gpg")
        .arg("--batch")
        .arg("--armor")
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
            .expect("failed to write to stdin");
    });
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!(anyhow!(
            "{}",
            String::from_utf8(output.stderr)?
                .trim()
                .rsplit_once("\n")
                .unwrap()
                .1
        ));
    }
    let parent = file.parent().unwrap();
    if !parent.exists() {
        debug!("creating dirs: {parent:?}");
        fs::create_dir_all(parent)?
    }
    debug!("writing contents to file: {file:?}");
    Ok(fs::write(file, output.stdout)?)
}

pub fn decrypt(passphrase: &str, file: &PathBuf) -> anyhow::Result<String> {
    if !file.exists() {
        bail!(Error::new(
            ErrorKind::NotFound,
            format!("file {file:#?} not found")
        ))
    }
    let output = Command::new("gpg")
        .arg("--batch")
        .arg("--passphrase")
        .arg(passphrase)
        .arg("--decrypt")
        .arg(file)
        .output()?;
    if !output.status.success() {
        bail!(anyhow!(
            "{}",
            String::from_utf8(output.stderr)?
                .trim()
                .rsplit_once("\n")
                .unwrap()
                .1
        ));
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub fn edit(file: &PathBuf, passphrase: &str) -> anyhow::Result<()> {
    let data = decrypt(passphrase, file).unwrap_or_default();
    let buffer = edit::edit(&data)?
        .split("\n")
        .map(|x| x.trim().to_owned() + "\n")
        .collect::<String>()
        .trim()
        .to_owned();
    if data == buffer {
        debug!("buffer unchanged");
        warn!("{file:#?} unchanged");
    } else {
        debug!("buffer edited");
        encrypt(passphrase, buffer.as_bytes(), file)?;
    }
    Ok(())
}
