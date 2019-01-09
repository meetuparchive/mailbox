use humantime::{Duration as HumanDuration, Timestamp};
use mailbox::Client;
use std::{error::Error as StdError, io::BufRead, ops::Add, str::FromStr, time::SystemTime};
use structopt::StructOpt;

#[derive(Debug, PartialEq)]
pub struct Time(SystemTime);

impl FromStr for Time {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Time(
            Timestamp::from_str(s)
                .map(|ts| ts.into())
                .or_else(|_| HumanDuration::from_str(s).map(|d| SystemTime::now().add(d.into())))
                .map_err(|e| format!("{}", e))?,
        ))
    }
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<StdError>>
where
    T: FromStr,
    T::Err: StdError + 'static,
    U: FromStr,
    U::Err: StdError + 'static,
{
    let pos = s
        .find(':')
        .ok_or_else(|| format!("invalid KEY:value: no `:` found in `{}`", s))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

#[derive(StructOpt, PartialEq, Debug)]
#[structopt(name = "mailbox", about = "IMap query client")]
struct Options {
    #[structopt(short = "u", long = "username", help = "Username to authenticate as")]
    username: String,
    #[structopt(
        short = "p",
        long = "password",
        default_value = "-",
        help = "Password to authenticate with"
    )]
    password: String,
    #[structopt(
        short = "d",
        long = "domain",
        default_value = "imap.gmail.com",
        help = "IMap server domain"
    )]
    domain: String,
    #[structopt(
        short = "P",
        long = "port",
        default_value = "993",
        help = "IMap server port"
    )]
    port: u16,
    #[structopt(
        short = "b",
        long = "box",
        default_value = "INBOX",
        help = "IMap mailbox name"
    )]
    mailbox: String,
    #[structopt(
        short = "w",
        long = "wait",
        help = "Time client will wait until message is found"
    )]
    wait: Option<Time>,
    #[structopt(
        short = "t",
        long = "template",
        help = "Template transformation (in handlebars) format"
    )]
    template: Option<String>,
    #[structopt(
        parse(try_from_str = "parse_key_val"),
        help = "key:value query pairs used to search and find messages. examples from:foo@bar.com subject:hello"
    )]
    #[structopt(required = true)]
    query: Vec<(String, String)>,
}

fn password<R>(
    provided: &str,
    read: &mut R,
) -> String
where
    R: BufRead,
{
    match provided {
        "-" => {
            let mut piped = String::new();
            read.read_line(&mut piped)
                .expect("failed to read password from stdin");
            piped
        }
        arg => arg.to_owned(),
    }
}

fn main() {
    let options = Options::from_args();
    let client = Client {
        username: options.username,
        password: password(&options.password, &mut std::io::stdin().lock()),
        domain: options.domain,
        port: options.port,
    };
    match client.find(options.mailbox, &options.query, options.wait.map(|w| w.0)) {
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
        Ok(messages) => {
            if messages.is_empty() {
                eprintln!("no messages found");
                std::process::exit(2);
            }
            let output = options
                .template
                .and_then(|tmpl| {
                    match handlebars::Handlebars::new().render_template(&tmpl, &messages) {
                        Err(e) => {
                            eprintln!("{}", e);
                            std::process::exit(3);
                        }
                        Ok(e) => Some(e),
                    }
                })
                .unwrap_or_else(|| {
                    serde_json::to_string_pretty(&messages).expect("failed to render json")
                });
            println!("{}", output);
        }
    }
}
