use core::panic;
use gpgme::{Context, Protocol};
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
    decrypt(passfile).trim().to_string()
}

pub fn encrypt(passphrase: &str, data: &[u8], file: &PathBuf) {
    // can't get gpg me to not ask for password
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
    if !output.stderr.is_empty() {
        panic!(
            "Encryption failed: {0}",
            String::from_utf8(output.stderr).unwrap()
        );
    }
    match fs::write(file, output.stdout) {
        Ok(_) => {}
        Err(error) => {
            panic!("Unable to write file: {error}")
        }
    }
}

pub fn decrypt(file: &PathBuf) -> String {
    let mut context =
        Context::from_protocol(Protocol::OpenPgp).expect("Was not able to create OpenPGP context");
    let mut text = Vec::new();
    match context.decrypt(fs::read(file).unwrap(), &mut text) {
        Ok(_) => String::from_utf8(text).unwrap(),
        Err(error) => {
            panic!("Decryption failed: {error}")
        }
    }
}
