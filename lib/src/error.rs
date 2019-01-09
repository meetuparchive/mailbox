use failure::Fail;
use imap::error::Error as IMapError;
use native_tls::TlsStream;
use std::{error::Error as StdError, fmt, net::TcpStream};

#[derive(Fail, Debug)]
pub enum Error {
    IMap(IMapError),
}

impl fmt::Display for Error {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        match self {
            Error::IMap(e) => write!(f, "{}", e.description()),
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
