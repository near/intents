//! Conversions between native outlayer types and their generated protobuf
//! counterparts.
//!
//! These live here, in the gRPC edge crate, rather than in each domain crate so
//! that the core crates stay free of any proto dependency. Both the native types
//! and the proto types are foreign to this crate, so the orphan rule forbids
//! `From`/`TryFrom` impls — we use local conversion traits instead.

use core::convert::Infallible;

use defuse_outlayer_executor::Outcome;
use defuse_outlayer_primitives::{AccountId, AppId};
use defuse_outlayer_proto as proto;
use defuse_outlayer_service::{AppCodeUrl, Code, CodeRef};
use defuse_outlayer_vm_runner::{ExecutionDetails, ExecutionOutcome};

use crate::ExecuteRequest;

pub trait ProtoTryFrom<P>: Sized {
    type Error;
    fn proto_try_from(value: P) -> Result<Self, Self::Error>;
}

impl ProtoTryFrom<proto::AppId> for AppId<'static> {
    type Error = anyhow::Error;
    fn proto_try_from(p: proto::AppId) -> Result<Self, Self::Error> {
        match p
            .variant
            .ok_or_else(|| anyhow::anyhow!("missing AppId variant"))?
        {
            proto::app_id::Variant::Near(s) => Ok(Self::from(s.parse::<AccountId>()?)),
        }
    }
}

impl ProtoTryFrom<proto::AppCodeUrl> for AppCodeUrl {
    type Error = anyhow::Error;
    fn proto_try_from(p: proto::AppCodeUrl) -> Result<Self, Self::Error> {
        let code_url = p.code_url.parse()?;
        let code_hash: [u8; 32] = p.code_hash.as_slice().try_into().map_err(|_| {
            anyhow::anyhow!("code_hash must be 32 bytes, got {}", p.code_hash.len())
        })?;
        Ok(Self {
            code_url,
            code_hash,
        })
    }
}

impl ProtoTryFrom<proto::CodeRef> for CodeRef<'static> {
    type Error = anyhow::Error;
    fn proto_try_from(p: proto::CodeRef) -> Result<Self, Self::Error> {
        match p
            .variant
            .ok_or_else(|| anyhow::anyhow!("missing CodeRef variant"))?
        {
            proto::code_ref::Variant::AppId(app_id) => {
                Ok(Self::AppId(AppId::proto_try_from(app_id)?))
            }
            proto::code_ref::Variant::Url(url) => Ok(Self::Url(AppCodeUrl::proto_try_from(url)?)),
        }
    }
}

impl ProtoTryFrom<proto::Code> for Code<'static> {
    type Error = anyhow::Error;
    fn proto_try_from(p: proto::Code) -> Result<Self, Self::Error> {
        match p
            .variant
            .ok_or_else(|| anyhow::anyhow!("missing Code variant"))?
        {
            proto::code::Variant::CodeRef(code_ref) => {
                Ok(Self::Ref(CodeRef::proto_try_from(code_ref)?))
            }
            proto::code::Variant::InlineCode(bytes) => Ok(Self::Inline { code: bytes.into() }),
        }
    }
}

impl ProtoTryFrom<proto::ExecuteRequest> for ExecuteRequest {
    type Error = anyhow::Error;
    fn proto_try_from(p: proto::ExecuteRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            app: Code::proto_try_from(p.app.ok_or_else(|| anyhow::anyhow!("missing app"))?)?,
            input: p.input.into(),
            fuel: p.fuel,
        })
    }
}

impl ProtoTryFrom<proto::Request> for ExecuteRequest {
    type Error = anyhow::Error;
    fn proto_try_from(p: proto::Request) -> Result<Self, Self::Error> {
        match p
            .kind
            .ok_or_else(|| anyhow::anyhow!("missing request kind"))?
        {
            proto::request::Kind::Execute(req) => Self::proto_try_from(req),
        }
    }
}

impl ProtoTryFrom<ExecutionDetails> for proto::ExecutionDetails {
    type Error = Infallible;
    fn proto_try_from(value: ExecutionDetails) -> Result<Self, Self::Error> {
        Ok(Self {
            fuel_consumed: value.fuel_consumed,
        })
    }
}

impl ProtoTryFrom<ExecutionOutcome> for proto::ExecutionOutcome {
    type Error = Infallible;
    fn proto_try_from(value: ExecutionOutcome) -> Result<Self, Self::Error> {
        Ok(Self {
            details: Some(proto::ExecutionDetails::proto_try_from(value.details)?),
            error: value.error,
        })
    }
}

impl ProtoTryFrom<Outcome> for proto::ExecuteResponse {
    type Error = Infallible;
    fn proto_try_from(value: Outcome) -> Result<Self, Self::Error> {
        Ok(Self {
            output: value.output.into(),
            logs: value.logs.into(),
            execution: Some(proto::ExecutionOutcome::proto_try_from(value.execution)?),
        })
    }
}
