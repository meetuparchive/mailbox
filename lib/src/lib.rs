#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use log::debug;
use mailparse::{parse_mail, ParsedMail};
use native_tls::TlsConnector;
use serde::Serialize;
use std::{
    collections::HashMap,
    io::{Read, Write},
    time::SystemTime,
};

mod error;
pub use crate::error::Error;

pub struct Client {
    pub username: String,
    pub password: String,
    pub domain: String,
    pub port: u16,
}

// http://www.xeams.com/difference-envelope-header.htm

#[derive(Debug, Serialize, PartialEq)]
pub struct Message {
    pub headers: HashMap<String, String>,
    pub subject: Option<String>,
    pub date: Option<String>,
    pub to: Option<String>,
    pub from: Option<String>,
    pub body: Vec<Part>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Part {
    pub content_type: String,
    pub body: Option<String>,
}

impl From<ParsedMail<'_>> for Message {
    fn from(parsed: ParsedMail) -> Self {
        let headers = (&parsed.headers)
            .into_iter()
            .fold(HashMap::new(), |mut result, header| {
                result.insert(
                    header.get_key().unwrap_or_default(),
                    header.get_value().unwrap_or_default(),
                );
                result
            });

        let parts = if parsed.ctype.mimetype.starts_with("multipart/") {
            parsed.subparts
        } else {
            vec![parsed]
        };
        let body = parts.into_iter().fold(Vec::new(), |mut result, part| {
            let body = if part.ctype.mimetype.starts_with("text/") {
                part.get_body().ok()
            } else {
                part.get_body_raw().ok().map(|b| base64::encode(&b))
            };
            result.push(Part {
                content_type: part.ctype.mimetype,
                body,
            });
            result
        });

        Message {
            headers: headers.clone(),
            subject: headers.get("Subject").cloned(),
            date: headers.get("Date").cloned(),
            to: headers.get("To").cloned(),
            from: headers.get("From").cloned(),
            body,
        }
    }
}

impl Client {
    fn search<T, Q>(
        &self,
        session: &mut imap::Session<T>,
        query: Q,
        wait: Option<SystemTime>,
    ) -> Result<Vec<Message>, Error>
    where
        Q: AsRef<str>,
        T: Read + Write,
    {
        let messages = session
            .uid_search(query.as_ref())?
            .into_iter()
            .try_fold::<_, _, Result<Vec<Message>, Error>>(Vec::new(), |mut result, uid| {
                if let Some(fetch) = session.uid_fetch(uid.to_string(), "RFC822")?.iter().next() {
                    let parsed = parse_mail(fetch.body().unwrap_or_default())?;
                    result.push(Message::from(parsed));
                }
                Ok(result)
            })?;
        if messages.is_empty() {
            if let Some(deadline) = wait {
                if SystemTime::now() < deadline {
                    debug!("retrying...");
                    return self.search(session, query.as_ref(), wait);
                }
            }
        }
        Ok(messages)
    }

    fn fmt_query(query: &[(String, String)]) -> String {
        query
            .into_iter()
            .map(|(k, v)| format!("{} {}", k, v).trim().to_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn find<M>(
        &self,
        mailbox: M,
        query: &[(String, String)],
        wait: Option<SystemTime>,
    ) -> Result<Vec<Message>, Error>
    where
        M: AsRef<str>,
    {
        let tls = TlsConnector::builder().build()?;
        let client = imap::connect((self.domain.as_str(), self.port), &self.domain, &tls)?;
        let mut session = client.login(&self.username, &self.password)?;
        session.select(mailbox.as_ref())?;

        let formatted = Self::fmt_query(query);
        debug!("search query '{}'", formatted);
        let messages = self.search(&mut session, formatted, wait)?;

        session.close()?;
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::{Client, Message, Part};
    use maplit::hashmap;

    #[test]
    fn formats_query() {
        assert_eq!(
            Client::fmt_query(&[("foo".into(), "bar".into()), ("baz".into(), "".into())]),
            "foo bar baz"
        )
    }

    #[test]
    fn converts_messages() {
        let message = Message::from(
            mailparse::parse_mail(include_bytes!("../tests/data/sample.mail"))
                .expect("failed to parse mail message"),
        );

        assert_eq!(
            message,
            Message {
                headers: hashmap! {
                    "Subject".to_string() => "This is a test email".to_string(),
                    "Content-Type".to_string() => "multipart/alternative; boundary=foobar".to_string(),
                    "Date".to_string() => "Sun, 02 Oct 2016 07:06:22 -0700 (PDT)".to_string()
                },
                subject: Some("This is a test email".into()),
                to: None,
                from: None,
                date: Some("Sun, 02 Oct 2016 07:06:22 -0700 (PDT)".into()),
                body: vec![
                    Part {
                        content_type: "text/plain".into(),
                        body: Some(
                            "This is the plaintext version, in utf-8. Proof by Euro: €".into()
                        )
                    },
                    Part {
                         content_type: "text/html".into(),
                         body: Some(
                            "<html><body>This is the <b>HTML</b> version, in us-ascii. Proof by Euro: &euro;</body></html>\n".into()
                        )
                    }
                ]
            }
        )
    }
}
