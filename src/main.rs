use std::env::{args_os, var};
use std::path::Path;
use std::process::{exit, Command, Stdio};

use confy;
use directories::{BaseDirs, ProjectDirs};
use hostname::get;
use lettre::smtp::authentication::Credentials;
use lettre::{SmtpClient, Transport};
use lettre_email::EmailBuilder;
use mmrs;
use notify_rust::Notification;
use serde::{Deserialize, Serialize};
use twilio::{Client, OutboundMessage};

#[derive(Debug, Serialize, Deserialize)]
struct LKConfig {
    services: Vec<String>,
    twilio_account_id: String,
    twilio_auth_token: String,
    twilio_sender: String,
    twilio_receiver: String,
    email_from: String,
    email_to: String,
    email_smtp: String,
    email_username: String,
    email_password: String,
    mattermost_url: String,
    mattermost_channel: String,
}

impl Default for LKConfig {
    fn default() -> Self {
        LKConfig {
            services: vec!("Enter services you wish to send through (if blank or with this string the program will exit early) e.g.:".to_string(), "twilio".to_string(), "email".to_string(), "mattermost".to_string(), "system".to_string()),
            twilio_account_id: "account_id from Twilio".to_string(),
            twilio_auth_token: "auth_token from Twilio".to_string(),
            twilio_sender: "Enter the number provided by Twilio".to_string(),
            twilio_receiver: "Enter the phone number to send message to".to_string(),
            email_from: "Enter the email you want to send the message to".to_string(),
            email_to: "Enter the email you want it to appear from".to_string(),
            email_smtp: "Enter the smtp server e.g. smtp.gmail.com".to_string(),
            email_username: "Enter the username for your email account".to_string(),
            email_password: "Enter the password for your email account".to_string(),
            mattermost_url: "Enter the full MatterMost webhook url".to_string(),
            mattermost_channel: "Enter the channel on the MatterMost server to send a message to (use @USERNAME to send a DM)".to_string(),
        }
    }
}

fn main() {
    let all_args = args_os();
    let mut process_args = String::new();

    let proj_config = BaseDirs::new().expect("Error could not determine home directory");
    let config_dir = proj_config.config_dir();
    let proj_dir = ProjectDirs::from("rs", "lk", "lk").expect("Error could not determine the project directory");

    if !Path::is_file(Path::new(&format!(
        "{}/{}/LK.toml",
        config_dir.display(),
        proj_dir.project_path().display()
    ))) {
        let cfg: LKConfig = LKConfig::default();
        confy::store("LK", cfg).expect("Error saving config file");
        println!("\nError: Configuration did not exist, please update {}/{}/{} with your settings and auth tokens...exiting\n\n", config_dir.display(), proj_dir.project_path().display(), "LK.toml");
        exit(2);
    }

    let hostname = get()
        .unwrap_or(std::ffi::OsString::from("HOSTNAME_NOT_FOUND"))
        .into_string()
        .expect("Error converting hostname to string");
    let config: LKConfig = confy::load("LK").expect(&format!(
        "Error could not load config for LK from {}/{}/{}",
        config_dir.display(),
        proj_dir.project_path().display(),
        "LK.toml"
    ));

    if config.services == LKConfig::default().services || config.services.is_empty() {
        println!(
            "\nError: default account found, please update {}/{}/{} with your settings...exiting\n\n",
            config_dir.display(),
            proj_dir.project_path().display(),
            "LK.toml"
        );
        exit(3);
    }

    for arg in all_args {
        process_args += &arg.into_string().expect("Error parsing argument to string");
        process_args += " ";
    }

    let split_process_names: Vec<&str> = process_args.splitn(2, "lk ").collect();

    //Grab the $SHELL variable, if unreachable assume bash
    let shell = var("SHELL").unwrap_or("/bin/bash".to_string());

    let child_process = Command::new(shell)
        .arg("-c")
        .arg(split_process_names[1])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Command failed to start");

    let output = child_process
        .wait_with_output()
        .expect("Error running command failed");

    if config.services.contains(&"twilio".to_string()) {
        //Checking for all Twilio settings before attempting to send
        if config.twilio_account_id == LKConfig::default().twilio_account_id
            || config.twilio_auth_token == LKConfig::default().twilio_auth_token
            || config.twilio_receiver == LKConfig::default().twilio_receiver
            || config.twilio_sender == LKConfig::default().twilio_sender
        {
            println!("Error found default values in twilio fields, please update the config file at {}/{}/{}", config_dir.display(), proj_dir.project_path().display(), "LK.toml");
            exit(4);
        }

        let client = Client::new(&config.twilio_account_id, &config.twilio_auth_token);

        if output.status.success() {
            client
                .send_message(OutboundMessage::new(
                    &config.twilio_sender,
                    &config.twilio_receiver,
                    &format!(
                        "Your command {} on {} exited successfully!!",
                        &split_process_names[1], &hostname
                    ),
                ))
                .expect("Error sending success message to Twilio API");
        } else {
            client
                .send_message(OutboundMessage::new(
                    &config.twilio_sender,
                    &config.twilio_receiver,
                    &format!(
                        "Your command {} on {} exited with an error ({})",
                        &split_process_names[1],
                        &hostname,
                        output
                            .status
                            .code()
                            .expect("Error getting status code from command")
                    ),
                ))
                .expect("Error sending failure message to Twilio API");
        }
    }

    if config.services.contains(&"email".to_string()) {
        //Making sure all fields are filled in before trying to send the email
        if config.email_username == LKConfig::default().email_username
            || config.email_password == LKConfig::default().email_password
            || config.email_from == LKConfig::default().email_from
            || config.email_to == LKConfig::default().email_to
            || config.email_smtp == LKConfig::default().email_smtp
        {
            println!(
                "Error found default values in email fields, please update the config file at {}/{}/{}",
                config_dir.display(),
                proj_dir.project_path().display(),
                "LK.toml"
            );
            exit(5);
        }

        let creds = Credentials::new(config.email_username, config.email_password);

        if output.status.success() {
            let email = EmailBuilder::new()
                .to(config.email_to)
                .from(config.email_from)
                .body(
                    format!(
                        "Here is the output: \n\n\n{}",
                        String::from_utf8_lossy(&output.stdout)
                    )
                    .to_string(),
                )
                .subject(format!("Your job on {} has completed!", &hostname).to_string())
                .build()
                .expect("Error building email struct");

            let mut mailer = SmtpClient::new_simple(&config.email_smtp)
                .expect("Error opening SMTP connection")
                .credentials(creds)
                .transport();

            // Send the email
            let result = mailer.send(email.into());

            if !result.is_ok() {
                println!("Email refused to send: {:?}", result);
            }
        } else {
            let email = EmailBuilder::new()
                .to(config.email_to)
                .from(config.email_from)
                .body(
                    format!(
                        "Here is the output: \n\n\n{}",
                        String::from_utf8_lossy(&output.stderr)
                    )
                    .to_string(),
                )
                .subject(format!("Your job on {} has failed!", &hostname).to_string())
                .build()
                .expect("Error building email struct");

            let mut mailer = SmtpClient::new_simple(&config.email_smtp)
                .expect("Error opening STMP connection")
                .credentials(creds)
                .transport();

            // Send the email
            let result = mailer.send(email.into());

            if !result.is_ok() {
                println!("Email refused to send: {:?}", result);
            }
        }
    }

    if config.services.contains(&"mattermost".to_string()) {
        //Checking for all mattermost fields before attempting to send the message
        if config.mattermost_url == LKConfig::default().mattermost_url
            || config.mattermost_channel == LKConfig::default().mattermost_channel
        {
            println!(
                "Error found default values in email fields, please update the config file at {}/{}/{}",
                config_dir.display(),
                proj_dir.project_path().display(),
                "LK.toml"
            );
            exit(6);
        }

        let mut message: mmrs::MMBody = mmrs::MMBody::new();

        message.username = Some("LK".to_string());
        message.icon_url = Some(
            "https://i0.wp.com/stephenkneale.com/wp-content/uploads/2019/11/Knowledge20Head.png"
                .to_string(),
        );
        message.channel = Some(config.mattermost_channel);

        if output.status.success() {
            message.text = Some(format!(
                "Your job on {} has finished successfully!",
                &hostname
            ));
        } else {
            message.text = Some(format!(
                "Your job on {} has failed, here's the output of stderr:\n\n{}",
                &hostname,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let body = message
            .to_json()
            .expect("Error converting MatterMost body to json");

        let response = mmrs::send_message(&config.mattermost_url, body);

        match response {
            Ok(code) => {
                if code == 200 {
                } else {
                    println!(
                        "Error sending job status to MatterMost\n\tresponse code: {}\n\n",
                        code
                    );
                    exit(7);
                }
            }
            Err(e) => {
                println!("Error within HTTP Request:\n{}\n\n", e);
                exit(8);
            }
        }
    }

    if config.services.contains(&"system".to_string()) {
        if output.status.success() {
            Notification::new()
                .summary(&format!("Your job on {} finished successfully!", &hostname))
                .body("Woo! 🎉")
                .show()
                .expect("Error: Could not show system notification");
        } else {
            Notification::new()
                .summary(&format!("Your job on {} finished successfully!", &hostname))
                .body("Boo! 😿")
                .show()
                .expect("Error: Could not show system notification");
        }
    }
    //Check for other services here once added (IRC, ???)
}
