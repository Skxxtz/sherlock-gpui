//! The streaming request: `AnswerStream::start` spawns a tokio task that posts the request and
//! forwards text deltas over a channel. The UI drains the channel once per frame
//! (`drain_into`), so many tokens cost one repaint. Dropping the stream aborts the request.

use std::time::Instant;

use tokio::{sync::mpsc, task::JoinHandle};

use crate::ui::views::ask::{
    model::exchange::{Exchange, RemoteLimits, Usage},
    provider::{
        DEFAULT_TEMPERATURE,
        Message,
        Provider,
        errors::{header_u64, http_error, send_error},
    },
};

/// What the task sends to the UI.
#[derive(Debug)]
enum Event {
    Delta(String),
    Usage(Usage),
    Limits(RemoteLimits),
    Done,
    Error(String),
}

/// Result of one `drain_into`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drain {
    /// Nothing new since the last drain.
    Idle,
    /// Text was appended.
    Changed,
    /// The stream ended (done, failed or the task died); stop polling.
    Ended,
}

pub struct AnswerStream {
    rx: mpsc::UnboundedReceiver<Event>,
    task: JoinHandle<()>,
    started: Instant,
}

impl Drop for AnswerStream {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl AnswerStream {
    /// Starts the request on the ambient tokio runtime. `None` outside a runtime.
    pub fn start(provider: &Provider, messages: Vec<Message>) -> Option<Self> {
        let rt = tokio::runtime::Handle::try_current().ok()?;
        let (tx, rx) = mpsc::unbounded_channel();
        let provider = provider.clone();
        let task = rt.spawn(async move {
            let ev = match run(&provider, &messages, &tx).await {
                Ok(()) => Event::Done,
                Err(e) => Event::Error(e),
            };
            let _ = tx.send(ev);
        });
        Some(Self {
            rx,
            task,
            started: Instant::now(),
        })
    }

    /// Moves everything received so far into `ex`. Call once per frame.
    pub fn drain_into(&mut self, ex: &mut Exchange) -> Drain {
        let mut state = Drain::Idle;
        loop {
            match self.rx.try_recv() {
                Ok(Event::Delta(text)) => {
                    ex.push(&text);
                    state = Drain::Changed;
                }
                Ok(Event::Usage(usage)) => {
                    ex.usage = Some(usage);
                    state = Drain::Changed;
                }
                Ok(Event::Limits(limits)) => {
                    ex.limits = Some(limits);
                }
                Ok(Event::Done) => {
                    ex.finish(self.started.elapsed());
                    return Drain::Ended;
                }
                Ok(Event::Error(e)) => {
                    ex.fail(e);
                    return Drain::Ended;
                }
                Err(mpsc::error::TryRecvError::Empty) => return state,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    ex.fail("connection closed");
                    return Drain::Ended;
                }
            }
        }
    }
}

async fn run(
    provider: &Provider,
    messages: &[Message],
    tx: &mpsc::UnboundedSender<Event>,
) -> Result<(), String> {
    let api = provider.kind.api();
    let temperature = provider
        .temperature
        .unwrap_or(DEFAULT_TEMPERATURE)
        .clamp(0.0, 2.0);
    let url = api.url(provider.base());
    let body = api.body(&provider.model, messages, temperature)?;

    let mut req = reqwest::Client::new()
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body);
    if let Some(key) = &provider.api_key {
        req = req.bearer_auth(key);
    }

    let mut resp = req.send().await.map_err(|e| send_error(provider, &e))?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let retry_after = header_u64(resp.headers(), "retry-after");
        let text = resp.text().await.unwrap_or_default();
        return Err(http_error(provider, status, retry_after, &text));
    }
    let limits = RemoteLimits {
        requests: header_u64(resp.headers(), "x-ratelimit-remaining-requests").map(|v| v as u32),
        tokens: header_u64(resp.headers(), "x-ratelimit-remaining-tokens"),
    };
    if limits.requests.is_some() || limits.tokens.is_some() {
        let _ = tx.send(Event::Limits(limits));
    }

    // Bytes until the next newline; a chunk can end mid-line or mid-UTF-8 sequence.
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    while let Some(chunk) = resp.chunk().await.map_err(|e| send_error(provider, &e))? {
        buf.extend_from_slice(&chunk);
        let mut start = 0;
        while let Some(nl) = buf[start..].iter().position(|&b| b == b'\n') {
            let line = &buf[start..start + nl];
            start += nl + 1;
            let line = api.parse_line(line)?;
            if !line.delta.is_empty() && tx.send(Event::Delta(line.delta.into_owned())).is_err() {
                return Ok(()); // UI dropped the stream
            }
            if let Some(usage) = line.usage {
                let _ = tx.send(Event::Usage(usage));
            }
            if line.end {
                return Ok(());
            }
        }
        buf.drain(..start);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::views::ask::model::exchange::Phase;

    #[test]
    fn drain_coalesces() {
        let (tx, rx) = mpsc::unbounded_channel();
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let task = rt.spawn(async {});
        let mut s = AnswerStream {
            rx,
            task,
            started: Instant::now(),
        };
        let mut ex = Exchange::new("q");
        assert_eq!(s.drain_into(&mut ex), Drain::Idle);
        for t in ["Run ", "`ss`. ", "Done"] {
            tx.send(Event::Delta(t.into())).unwrap();
        }
        assert_eq!(s.drain_into(&mut ex), Drain::Changed);
        assert_eq!(ex.text(), "Run `ss`. Done");
        tx.send(Event::Usage(Usage {
            input: 5,
            output: 3,
        }))
        .unwrap();
        tx.send(Event::Done).unwrap();
        assert_eq!(s.drain_into(&mut ex), Drain::Ended);
        assert!(matches!(ex.phase, Phase::Done { .. }));
        assert_eq!(
            ex.usage,
            Some(Usage {
                input: 5,
                output: 3
            })
        );
    }
}
