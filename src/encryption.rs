use core::panic;
use scanpw::scanpw;
use std::io::Write;
use std::process::{Command, Stdio};
use std::{fs, path::PathBuf};

pub fn set_passphrase(passfile: &PathBuf) -> String {
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
    encrypt(pass1.trim(), pass1.trim().as_bytes(), passfile);
    decrypt(passfile, None).unwrap()
}

pub fn encrypt(passphrase: &str, data: &[u8], file: &PathBuf) {
    let mut child = Command::new("gpg")
        .arg("--batch")
        .arg("--passphrase")
        .arg(passphrase)
        .arg("--symmetric")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn gpg process");
    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let data_clone = Vec::from(data);
    std::thread::spawn(move || {
        stdin
            .write_all(&data_clone)
            .expect("Failed to write to stdin");
    });
    let output = child.wait_with_output().expect("Failed to read stdout");
    if !output.status.success() {
        panic!("{0}", String::from_utf8(output.stderr).unwrap());
    }
    let parent = file.parent().unwrap();
    if !parent.exists() {
        match fs::create_dir_all(parent) {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create parent dirs: {error}")
            }
        }
    }
    match fs::write(file, output.stdout) {
        Ok(_) => {}
        Err(error) => {
            panic!("Unable to write file: {error}")
        }
    }
}

pub fn decrypt(file: &PathBuf, passphrase: Option<&str>) -> Option<String> {
    if !file.exists() {
        return None;
    }
    let output = if let Some(passphrase) = passphrase {
        Command::new("gpg")
            .arg("--batch")
            .arg("--passphrase")
            .arg(passphrase)
            .arg("--decrypt")
            .arg(file)
            .output()
            .expect("Failed to spawn gpg process")
    } else {
        Command::new("gpg")
            .arg("--decrypt")
            .arg(file)
            .output()
            .expect("Failed to spawn gpg process")
    };
    if !output.status.success() {
        panic!("{0}", String::from_utf8(output.stderr).unwrap());
    }
    Some(String::from_utf8(output.stdout).unwrap())
}
