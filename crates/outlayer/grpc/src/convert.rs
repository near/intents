//! Conversions between native outlayer types and their generated protobuf
//! counterparts.
//!
//! These live here, in the gRPC edge crate, rather than in each domain crate so
//! that the core crates stay free of any proto dependency. Both the native types
//! and the proto types are foreign to this crate, so the orphan rule forbids
//! `From`/`TryFrom` impls — we use local conversion traits instead.

use defuse_outlayer_executor::Outcome;
use defuse_outlayer_primitives::{AccountId, AppId};
use defuse_outlayer_proto as proto;
use defuse_outlayer_service::{AppCodeUrl, Code, CodeRef, ExecuteRequest};
use defuse_outlayer_vm_runner::{ExecutionDetails, ExecutionOutcome};

pub trait TryFromProto<P>: Sized {
    fn try_from_proto(proto: P) -> anyhow::Result<Self>;
}

pub trait IntoProto<P> {
    fn into_proto(self) -> P;
}

impl TryFromProto<proto::AppId> for AppId<'static> {
    fn try_from_proto(p: proto::AppId) -> anyhow::Result<Self> {
        match p
            .variant
            .ok_or_else(|| anyhow::anyhow!("missing AppId variant"))?
        {
            proto::app_id::Variant::Near(s) => Ok(Self::from(s.parse::<AccountId>()?)),
        }
    }
}

impl TryFromProto<proto::AppCodeUrl> for AppCodeUrl {
    fn try_from_proto(p: proto::AppCodeUrl) -> anyhow::Result<Self> {
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

impl TryFromProto<proto::CodeRef> for CodeRef<'static> {
    fn try_from_proto(p: proto::CodeRef) -> anyhow::Result<Self> {
        match p
            .variant
            .ok_or_else(|| anyhow::anyhow!("missing CodeRef variant"))?
        {
            proto::code_ref::Variant::AppId(app_id) => {
                Ok(Self::AppId(AppId::try_from_proto(app_id)?))
            }
            proto::code_ref::Variant::Url(url) => Ok(Self::Url(AppCodeUrl::try_from_proto(url)?)),
        }
    }
}

impl TryFromProto<proto::Code> for Code<'static> {
    fn try_from_proto(p: proto::Code) -> anyhow::Result<Self> {
        match p
            .variant
            .ok_or_else(|| anyhow::anyhow!("missing Code variant"))?
        {
            proto::code::Variant::CodeRef(code_ref) => {
                Ok(Self::Ref(CodeRef::try_from_proto(code_ref)?))
            }
            proto::code::Variant::InlineCode(bytes) => Ok(Self::Inline { code: bytes.into() }),
        }
    }
}

impl TryFromProto<proto::ExecuteRequest> for ExecuteRequest {
    fn try_from_proto(p: proto::ExecuteRequest) -> anyhow::Result<Self> {
        Ok(Self {
            app: Code::try_from_proto(p.app.ok_or_else(|| anyhow::anyhow!("missing app"))?)?,
            input: p.input.into(),
            fuel: p.fuel,
        })
    }
}

impl TryFromProto<proto::Request> for ExecuteRequest {
    fn try_from_proto(p: proto::Request) -> anyhow::Result<Self> {
        match p
            .kind
            .ok_or_else(|| anyhow::anyhow!("missing request kind"))?
        {
            proto::request::Kind::Execute(req) => Self::try_from_proto(req),
        }
    }
}

impl IntoProto<proto::ExecutionDetails> for ExecutionDetails {
    fn into_proto(self) -> proto::ExecutionDetails {
        proto::ExecutionDetails {
            fuel_consumed: self.fuel_consumed,
        }
    }
}

impl IntoProto<proto::ExecutionOutcome> for ExecutionOutcome {
    fn into_proto(self) -> proto::ExecutionOutcome {
        proto::ExecutionOutcome {
            details: Some(self.details.into_proto()),
            error: self.error,
        }
    }
}

impl IntoProto<proto::ExecuteResponse> for Outcome {
    fn into_proto(self) -> proto::ExecuteResponse {
        proto::ExecuteResponse {
            output: self.output.into(),
            logs: self.logs.into(),
            execution: Some(self.execution.into_proto()),
        }
    }
}
