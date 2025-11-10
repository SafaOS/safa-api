#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
extern crate alloc;

use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
#[cfg(feature = "std")]
use std as alloc;

use alloc::string::String;
use safa_abi::{errors::ErrorStatus, sockets::SockMsgFlags};
use simpldns::message::{
    DnsClass, DnsMessage, DnsMessageFlags, DnsMessageHeader, DnsOpCode, DnsQuestion, DnsRCode,
    DnsType, RRData,
};

use crate::{
    sockets::{socket::SocketOpt, Socket, SocketDomain, SocketKind},
    syscalls,
};

#[inline]
fn get_nameserver() -> SocketAddrV4 {
    // TODO: actually read nameserver
    SocketAddrV4::new(Ipv4Addr::new(1, 1, 1, 1), 53)
}

fn send_and_recv<'a>(
    send: &[u8],
    encode_to: &'a mut [u8],
    mut retries: usize,
    timeout_ms: u64,
) -> Result<&'a [u8], ErrorStatus> {
    let send_to = get_nameserver();

    let bind_to = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);

    let socket = Socket::builder(SocketDomain::Ipv4, SocketKind::Datagram, 0).build()?;
    socket.set_sock_opt(SocketOpt::ReadTimeout, timeout_ms)?;
    socket.bind_to_addr(bind_to)?;

    let send_me = || socket.send_to_addr(send, SockMsgFlags::NONE, SocketAddr::V4(send_to));
    send_me()?;

    loop {
        let results = socket.recv_from_addr(encode_to, SockMsgFlags::NONE);
        break match results {
            Ok((recv, addr)) => {
                if addr != send_to {
                    // recv again without counting this as a retry
                    continue;
                }

                Ok(&encode_to[..recv])
            }
            Err(e @ ErrorStatus::Timeout) => {
                if retries == 0 {
                    break Err(e);
                }

                // buf didn't change
                send_me()?;
                retries -= 1;
                continue;
            }
            Err(e) => Err(e),
        };
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DnsResolutionError {
    NoResponse,
    NoSuchName,
    Refused,
    InvalidDomainName,
    System(ErrorStatus),
}

impl From<ErrorStatus> for DnsResolutionError {
    fn from(value: ErrorStatus) -> Self {
        match value {
            ErrorStatus::Timeout => Self::NoResponse,
            e => Self::System(e),
        }
    }
}

pub fn lookup_dns<F>(domain: &str, mut with_result: F) -> Result<Option<String>, DnsResolutionError>
where
    F: FnMut(Ipv4Addr),
{
    // TODO: random numbers
    let trans_id = syscalls::misc::uptime() as u16;
    let questions = [
        DnsQuestion::try_new(domain, DnsType::A /* TODO: Ipv6? */, DnsClass::IN)
            .map_err(|_| DnsResolutionError::InvalidDomainName)?,
    ];

    let msg = DnsMessage::new(DnsMessageHeader::new(
        trans_id,
        DnsOpCode::Query,
        DnsRCode::NoError,
        DnsMessageFlags::QUERY | DnsMessageFlags::RECURSION_DESIRED,
    ))
    .with_questions(&questions);

    let mut encode_buf = [0u8; 512];
    msg.encode_to(&mut encode_buf)
        .expect("Encoding the message shall not fail");

    let mut resp_buf = [0u8; 512];
    let response_msg = send_and_recv(&encode_buf, &mut resp_buf, 3, 300)?;
    let message =
        DnsMessage::parse(response_msg).expect("DNS nameserver returned an invalid message");

    match message.header().rcode() {
        DnsRCode::FormatError => unreachable!("We encoded a bad DNS message"),
        DnsRCode::NameError => return Err(DnsResolutionError::NoSuchName),
        DnsRCode::Refused | DnsRCode::NotImplemented | DnsRCode::ServerFailure => {
            return Err(DnsResolutionError::Refused)
        }
        DnsRCode::NoError => {}
    }

    let mut name_buf = [0u8; 512];
    let answers = message.answers();

    let mut cname = None;

    for ans in answers {
        match ans.rdata() {
            RRData::A(a) => with_result(*a),
            RRData::CName(canon_name) => {
                let mut cursor = 0;
                for n in *canon_name {
                    let len = n.len() as usize;
                    name_buf[cursor..cursor + len].copy_from_slice(n.as_bytes());

                    cursor += len;
                    name_buf[cursor] = b'.';
                    cursor += 1;
                }

                let name_str = core::str::from_utf8(&name_buf[..cursor]).unwrap();
                cname = Some(name_str);
            }
            _ => {}
        }
    }
    Ok(cname.filter(|c| *c != domain).map(|c| String::from(c)))
}
