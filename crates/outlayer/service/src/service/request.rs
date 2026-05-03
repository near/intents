use bytes::Bytes;

use crate::resolver::App;

pub struct Request<'a> {
    pub app: App<'a>,
    pub input: Bytes,
    pub fuel: u64,
}
