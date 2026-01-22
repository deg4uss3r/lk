# lk (lemme know) - Sends you a Message when a Command has finished (with success or failure status)

## Example

![](./objects/demo.gif)

## Details

To configure go to the `$XDG` base directory folder path (MacOS: `~/.config/lk/default-config.toml`,  Linux: `home/$USER/.config/LK/default-config.toml`) and adjust the configuration file to the services you want to be sent a message to (supports email, MatterMost, and Twilio at the current time).

Once done make sure your config file is set to `600` or `rw---` so you are the only one able to see and edit the file with passwords, and API tokens!!

Right now `lk` supports sending messages via Twilio API, Email, MatterMost API, or System notifications. Have any more suggestions, put in an issue or PRs are welcome.

## Install

To install:

`cargo install lemmeknow`

This will install an `lk` binary for you to use.
