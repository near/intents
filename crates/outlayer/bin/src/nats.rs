use async_nats::{connect, Client, Message, Subscriber};
use anyhow::Result;
use futures::{Stream, StreamExt as _};
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct NatsConnector {
    subscriber: Subscriber,
}


#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize)
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields, default))]
pub struct NatsConfig {
    url: String,
    topic: String,
}


impl Default for NatsConfig{
    fn default() -> Self {
        NatsConfig {
            url: "nats://nats:4222".to_string(),
            topic: "requests".to_string(),
        }
    }
}

impl Stream for NatsConnector {
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.subscriber.poll_next_unpin(cx)
    }
}

impl NatsConnector {
    pub async fn connect(config: NatsConfig) -> Result<Self> {
        let nats = connect(config.url).await?;
        let subscriber = nats.subscribe(config.topic).await?;
        Ok(NatsConnector { subscriber })
    }

}
