use failure::Fail;
use imap::error::Error as IMapError;
use mailparse::MailParseError;
use native_tls::TlsStream;
use std::{error::Error as StdError, fmt, net::TcpStream};

#[derive(Fail, Debug)]
pub enum Error {
    IMap(IMapError),
    Parse(MailParseError),
}

impl fmt::Display for Error {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        match self {
            Error::IMap(e) => write!(f, "{}", e.description()),
            Error::Parse(e) => write!(f, "{}", e.description()),
        }
    }
}

impl From<(IMapError, imap::Client<TlsStream<TcpStream>>)> for Error {
    fn from(e: (IMapError, imap::Client<TlsStream<TcpStream>>)) -> Self {
        let (imap, _) = e;
        Error::from(imap)
    }
}

impl From<IMapError> for Error {
    fn from(e: IMapError) -> Self {
        Error::IMap(e)
    }
}

impl From<MailParseError> for Error {
    fn from(e: MailParseError) -> Self {
        Error::Parse(e)
    }
}
