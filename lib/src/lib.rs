use mailparse::parse_mail;
use native_tls::TlsConnector;
use serde::Serialize;
use std::{collections::HashMap, time::SystemTime};

mod error;
pub use crate::error::Error;

pub struct Client {
    pub username: String,
    pub password: String,
    pub domain: String,
    pub port: u16,
}

// http://www.xeams.com/difference-envelope-header.htm

#[derive(Debug, Serialize)]
pub struct Message {
    pub headers: HashMap<String, String>,
    pub subject: Option<String>,
    pub date: Option<String>,
    pub to: Option<String>,
    pub from: Option<String>,
    pub body: Vec<Part>,
}

#[derive(Debug, Serialize)]
pub struct Part {
    pub content_type: String,
    pub body: Option<String>,
}

impl Client {
    pub fn find<M>(
        &self,
        mailbox: M,
        query: &Vec<(String, String)>,
        wait: Option<SystemTime>,
    ) -> Result<Vec<Message>, Error>
    where
        M: AsRef<str>,
    {
        let tls = TlsConnector::builder().build().unwrap();
        let client = imap::connect((self.domain.as_str(), self.port), &self.domain, &tls)?;
        let mut session = client.login(&self.username, &self.password)?;
        session.select(mailbox.as_ref())?;

        let search = query
            .into_iter()
            .map(|(k, v)| format!("{} {}", k, v))
            .collect::<Vec<_>>()
            .join(" ");
        let mut messages = vec![];
        for s in session.uid_search(search)? {
            if let Some(fetch) = session.uid_fetch(s.to_string(), "RFC822")?.iter().next() {
                let parsed = parse_mail(fetch.body().unwrap_or_default()).unwrap();

                let body = if parsed.ctype.mimetype.starts_with("multipart/") {
                    parsed
                        .subparts
                        .into_iter()
                        .fold(Vec::new(), |mut result, part| {
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
                        })
                } else {
                    let body = if parsed.ctype.mimetype.starts_with("text/") {
                        parsed.get_body().ok()
                    } else {
                        parsed.get_body_raw().ok().map(|b| base64::encode(&b))
                    };
                    vec![Part {
                        content_type: parsed.ctype.mimetype,
                        body,
                    }]
                };
                let headers =
                    parsed
                        .headers
                        .into_iter()
                        .fold(HashMap::new(), |mut result, header| {
                            result.insert(header.get_key().unwrap(), header.get_value().unwrap());
                            result
                        });
                messages.push(Message {
                    headers: headers.clone(),
                    subject: headers.get("Subject").cloned(),
                    date: headers.get("Date").cloned(),
                    to: headers.get("To").cloned(),
                    from: headers.get("From").cloned(),
                    body,
                })
            }
        }
        session.close()?;
        if messages.is_empty() {
            if let Some(deadline) = wait {
                if SystemTime::now() < deadline {
                    return self.find(
                        mailbox,
                        query,
                        wait
                    );
                }
            }
        }
        Ok(messages)
    }
}
