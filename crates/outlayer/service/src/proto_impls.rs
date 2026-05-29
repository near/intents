use defuse_outlayer_proto as proto;
use url::Url;

use crate::{AppCodeUrl, Code, CodeRef};

impl TryFrom<proto::AppCodeUrl> for AppCodeUrl {
    type Error = anyhow::Error;

    fn try_from(p: proto::AppCodeUrl) -> Result<Self, Self::Error> {
        let code_url = p.code_url.parse::<Url>()?;
        let code_hash: [u8; 32] = p.code_hash.as_slice().try_into().map_err(|_| {
            anyhow::anyhow!("code_hash must be 32 bytes, got {}", p.code_hash.len())
        })?;
        Ok(Self {
            code_url,
            code_hash,
        })
    }
}

impl TryFrom<proto::CodeRef> for CodeRef<'static> {
    type Error = anyhow::Error;

    fn try_from(p: proto::CodeRef) -> Result<Self, Self::Error> {
        match p
            .variant
            .ok_or_else(|| anyhow::anyhow!("missing CodeRef variant"))?
        {
            proto::code_ref::Variant::AppId(app_id) => Ok(Self::AppId(app_id.try_into()?)),
            proto::code_ref::Variant::Url(url) => Ok(Self::Url(url.try_into()?)),
        }
    }
}

impl TryFrom<proto::Code> for Code<'static> {
    type Error = anyhow::Error;

    fn try_from(p: proto::Code) -> Result<Self, Self::Error> {
        match p
            .variant
            .ok_or_else(|| anyhow::anyhow!("missing Code variant"))?
        {
            proto::code::Variant::CodeRef(code_ref) => Ok(Self::Ref(code_ref.try_into()?)),
            proto::code::Variant::InlineCode(bytes) => Ok(Self::Inline { code: bytes.into() }),
        }
    }
}
