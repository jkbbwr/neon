//! The Neon language server.
//!
//! Two capabilities, both of which the compiler already supports honestly: diagnostics,
//! and formatting. Nothing else is advertised. An editor that is told a server can do
//! hover or go-to-definition, and then gets nothing back, looks broken in a way the user
//! cannot diagnose — so a capability appears here only once it works.
//!
//! Go-to-definition is the obvious next one and the architecture already accommodates it:
//! the stdlib is loaded from real files with real spans precisely so that jumping into
//! `println` can open one (`docs/decisions.md`). What is missing is a span-to-definition
//! index, not access to the source.
//!
//! The loop is synchronous and handles one message at a time. That is affordable because
//! the two expensive things have been taken off the per-keystroke path: the stdlib is
//! parsed once per session (`analysis::Analyzer`), and checks are debounced so a burst of
//! typing costs one check rather than one per character.

mod analysis;
mod position;
mod toolchain;

use lsp_server::{Connection, ExtractError, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    LogMessage, Notification as _, PublishDiagnostics, ShowMessage,
};
use lsp_types::request::{Formatting, Request as _};
use lsp_types::{
    DiagnosticRelatedInformation, DiagnosticSeverity, DocumentFormattingParams, InitializeParams,
    Location, LogMessageParams, MessageType, OneOf, PublishDiagnosticsParams, Range,
    ServerCapabilities, ShowMessageParams, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextEdit, Url,
};
use position::LineIndex;
use std::collections::HashMap;
use std::error::Error;
use std::time::{Duration, Instant};

/// How long the server waits for typing to stop before checking.
///
/// A check is a blocking pass over the user's module, and a keystroke invalidates the one
/// in flight anyway, so running per character spends the whole budget on results nobody
/// reads. 150ms is below the threshold where a pause feels like lag and comfortably above
/// a fast typist's inter-key interval, so ordinary typing produces exactly one check when
/// the user stops.
const DEBOUNCE: Duration = Duration::from_millis(150);

/// The longest a document may go unchecked while still being edited.
///
/// Without this, continuous typing renews the debounce forever and diagnostics never
/// appear at all — the failure mode where the feature looks broken rather than slow.
const MAX_DELAY: Duration = Duration::from_millis(750);

/// Open documents, by URI. The editor's copy is authoritative — a file on disk may be
/// stale by several keystrokes, and checking the stale one would report errors the user
/// has already fixed.
type Documents = HashMap<Url, LineIndex>;

/// A document that has changed and not yet been checked.
struct Pending {
    uri: Url,
    /// When it first went dirty — the anchor for `MAX_DELAY`.
    first: Instant,
    /// When it last changed — the anchor for `DEBOUNCE`.
    last: Instant,
}

impl Pending {
    /// The moment the check should run: whichever of the two limits comes first.
    fn deadline(&self) -> Instant {
        (self.last + DEBOUNCE).min(self.first + MAX_DELAY)
    }
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    // stdio: the transport every editor supports without configuration.
    let (connection, io_threads) = Connection::stdio();

    let capabilities = serde_json::to_value(ServerCapabilities {
        // Full sync: the checker needs a whole module anyway, so applying incremental
        // edits would only be bookkeeping on the way to reassembling the same string.
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        document_formatting_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })?;

    let init = connection.initialize(capabilities)?;
    let _params: InitializeParams = serde_json::from_value(init)?;

    let analyzer = start_analyzer(&connection)?;
    serve(&connection, &analyzer)?;

    // Dropping the connection first is required, not tidiness: `join` waits for the
    // writer thread, and that thread only finishes once its end of the channel is
    // disconnected. Holding `connection` across the join deadlocks the server after a
    // clean shutdown — the editor gets its response and the process never exits, leaking
    // one per session.
    drop(connection);
    io_threads.join()?;
    Ok(())
}

/// Load the toolchain's stdlib, telling the user whatever happened.
///
/// A server with no stdlib can still lex and parse, which is worth having, so this
/// degrades rather than refusing to start — but it does not degrade *quietly*. The old
/// behaviour was to fall back in silence, which left the user looking at a file with no
/// diagnostics that would not compile, and nothing anywhere to explain why. That is worse
/// than either extreme: a clean file is an assertion, and the server was making one it
/// could not support.
///
/// So both channels are used, for different readers. `window/logMessage` gets the full
/// detail and lands in the editor's output pane, which is where someone debugging a setup
/// looks; it is sent on success too, so "which stdlib am I actually checking against" has
/// an answer without reproducing a failure. `window/showMessage` fires only on trouble and
/// only once, because the user has to be interrupted at least once — nobody opens the log
/// pane to discover a problem they do not yet know they have — and a popup per keystroke
/// would train them to dismiss it unread.
fn start_analyzer(
    connection: &Connection,
) -> Result<analysis::Analyzer, Box<dyn Error + Sync + Send>> {
    let (analyzer, log, warn) = match toolchain::load() {
        Ok(std) => {
            let detail = format!(
                "neon-lsp: loaded {} stdlib file(s) from '{}'; type checking is on.",
                std.sources.len(),
                std.dir.display()
            );
            match analysis::Analyzer::new(&std.sources) {
                Ok(a) => (a, detail, None),
                // A stdlib that does not parse is a broken toolchain, not the user's
                // fault — but they still need to know why type errors stopped appearing.
                Err(e) => {
                    let msg = format!(
                        "neon-lsp: the stdlib at '{}' did not parse, so only syntax errors \
                         will appear. {e}",
                        std.dir.display()
                    );
                    (analysis::Analyzer::syntax_only(), msg.clone(), Some(msg))
                }
            }
        }
        Err(failure) => {
            let msg = failure.message();
            (analysis::Analyzer::syntax_only(), msg.clone(), Some(msg))
        }
    };

    notify(
        connection,
        LogMessage::METHOD,
        LogMessageParams {
            typ: if warn.is_some() { MessageType::WARNING } else { MessageType::INFO },
            message: log,
        },
    )?;
    if let Some(message) = warn {
        notify(
            connection,
            ShowMessage::METHOD,
            ShowMessageParams { typ: MessageType::WARNING, message },
        )?;
    }
    Ok(analyzer)
}

fn notify<P: serde::Serialize>(
    connection: &Connection,
    method: &str,
    params: P,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let note = Notification::new(method.to_string(), params);
    connection.sender.send(Message::Notification(note))?;
    Ok(())
}

fn serve(
    connection: &Connection,
    analyzer: &analysis::Analyzer,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut docs: Documents = HashMap::new();
    let mut pending: Option<Pending> = None;

    loop {
        // With a check pending, wait only until its deadline; without one, wait forever.
        // This is the whole debounce: every message that arrives first renews the wait,
        // and the timeout firing is what makes the check happen. No threads and no
        // cancellation are needed, because the work never starts until the wait ends.
        let msg = match &pending {
            Some(p) => {
                let wait = p.deadline().saturating_duration_since(Instant::now());
                match connection.receiver.recv_timeout(wait) {
                    Ok(msg) => msg,
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        flush(connection, &docs, analyzer, pending.take())?;
                        continue;
                    }
                    // The editor vanished. Its diagnostics do not matter now.
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return Ok(()),
                }
            }
            None => match connection.receiver.recv() {
                Ok(msg) => msg,
                Err(_) => return Ok(()),
            },
        };

        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                // Answered immediately even mid-debounce: `docs` is updated on every
                // change, so a format never works from stale text. Only the *check* is
                // deferred.
                let response = handle_request(req, &docs);
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(note) => match document_event(note) {
                Some(Event::Changed(uri, text)) => {
                    docs.insert(uri.clone(), LineIndex::new(&text));
                    let now = Instant::now();
                    match &mut pending {
                        // Same document still settling: renew the debounce, but keep the
                        // original `first` so `MAX_DELAY` still bounds the total wait.
                        Some(p) if p.uri == uri => p.last = now,
                        // A different document went dirty. Check the old one now rather
                        // than letting edits elsewhere postpone it indefinitely.
                        Some(_) => {
                            flush(connection, &docs, analyzer, pending.take())?;
                            pending = Some(Pending { uri, first: now, last: now });
                        }
                        None => pending = Some(Pending { uri, first: now, last: now }),
                    }
                }
                Some(Event::Closed(uri)) => {
                    docs.remove(&uri);
                    if pending.as_ref().is_some_and(|p| p.uri == uri) {
                        pending = None;
                    }
                    // An editor keeps showing the last set it was told about, so a closed
                    // file would otherwise leave stale errors in the problems panel
                    // forever. Not debounced: closing is a deliberate act, not a burst.
                    connection.sender.send(Message::Notification(empty_diagnostics(&uri)))?;
                }
                None => {}
            },
            Message::Response(_) => {}
        }
    }
}

/// Run the deferred check and publish its result.
fn flush(
    connection: &Connection,
    docs: &Documents,
    analyzer: &analysis::Analyzer,
    pending: Option<Pending>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let Some(p) = pending else { return Ok(()) };
    // Closed between the edit and the deadline: nothing to say about it.
    let Some(index) = docs.get(&p.uri) else { return Ok(()) };
    connection.sender.send(Message::Notification(publish(&p.uri, index, analyzer)))?;
    Ok(())
}

/// What a text-document notification means to this server.
enum Event {
    Changed(Url, String),
    Closed(Url),
}

fn document_event(note: Notification) -> Option<Event> {
    match note.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let p: lsp_types::DidOpenTextDocumentParams = serde_json::from_value(note.params).ok()?;
            Some(Event::Changed(p.text_document.uri, p.text_document.text))
        }
        DidChangeTextDocument::METHOD => {
            let p: lsp_types::DidChangeTextDocumentParams =
                serde_json::from_value(note.params).ok()?;
            // Full sync, so the last change carries the whole document.
            let text = p.content_changes.into_iter().next_back()?.text;
            Some(Event::Changed(p.text_document.uri, text))
        }
        DidSaveTextDocument::METHOD => {
            let p: lsp_types::DidSaveTextDocumentParams = serde_json::from_value(note.params).ok()?;
            Some(Event::Changed(p.text_document.uri, p.text?))
        }
        DidCloseTextDocument::METHOD => {
            let p: lsp_types::DidCloseTextDocumentParams =
                serde_json::from_value(note.params).ok()?;
            Some(Event::Closed(p.text_document.uri))
        }
        _ => None,
    }
}

fn empty_diagnostics(uri: &Url) -> Notification {
    Notification::new(
        PublishDiagnostics::METHOD.to_string(),
        PublishDiagnosticsParams { uri: uri.clone(), diagnostics: Vec::new(), version: None },
    )
}

/// Check a document and build the notification carrying its diagnostics.
fn publish(uri: &Url, index: &LineIndex, analyzer: &analysis::Analyzer) -> Notification {
    let diagnostics = analyzer
        .diagnostics(index.text())
        .into_iter()
        .map(|d| lsp_types::Diagnostic {
            range: Range {
                start: index.position(d.span.start),
                end: index.position(d.span.end),
            },
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("neon".into()),
            message: match d.help {
                // The help text is the actionable half of most of these messages, and an
                // editor shows one string. Appending beats dropping it.
                Some(help) => format!("{}\n\nhelp: {help}", d.message),
                None => d.message,
            },
            related_information: (!d.labels.is_empty()).then(|| {
                d.labels
                    .into_iter()
                    .map(|(span, note)| DiagnosticRelatedInformation {
                        location: Location {
                            uri: uri.clone(),
                            range: Range {
                                start: index.position(span.start),
                                end: index.position(span.end),
                            },
                        },
                        message: note,
                    })
                    .collect()
            }),
            ..Default::default()
        })
        .collect::<Vec<_>>();

    Notification::new(
        PublishDiagnostics::METHOD.to_string(),
        PublishDiagnosticsParams { uri: uri.clone(), diagnostics, version: None },
    )
}

fn handle_request(req: Request, docs: &Documents) -> Response {
    match req.method.as_str() {
        Formatting::METHOD => match cast::<Formatting>(req) {
            Ok((id, params)) => format_document(id, params, docs),
            Err(err) => err,
        },
        _ => Response::new_err(
            req.id,
            lsp_server::ErrorCode::MethodNotFound as i32,
            format!("unhandled request: {}", req.method),
        ),
    }
}

/// Format a whole document, as one edit replacing everything.
///
/// A file that does not parse is left alone: the formatter reprints from the AST, so
/// there is nothing to reprint, and returning no edits is exactly right — the editor
/// leaves the buffer as the user typed it. Failing loudly here would mean an error popup
/// on every format-on-save while a line is half-written.
fn format_document(id: RequestId, params: DocumentFormattingParams, docs: &Documents) -> Response {
    let Some(index) = docs.get(&params.text_document.uri) else {
        return Response::new_ok(id, Vec::<TextEdit>::new());
    };
    let Ok(formatted) = neon_compiler::format::format(index.text()) else {
        return Response::new_ok(id, Vec::<TextEdit>::new());
    };
    if formatted == index.text() {
        return Response::new_ok(id, Vec::<TextEdit>::new());
    }
    let end = index.position(index.text().len());
    let edit = TextEdit {
        range: Range { start: lsp_types::Position { line: 0, character: 0 }, end },
        new_text: formatted,
    };
    Response::new_ok(id, vec![edit])
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), Response>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD).map_err(|e| match e {
        ExtractError::MethodMismatch(r) => Response::new_err(
            r.id,
            lsp_server::ErrorCode::MethodNotFound as i32,
            "method mismatch".into(),
        ),
        ExtractError::JsonError { method, error } => Response::new_err(
            RequestId::from(0),
            lsp_server::ErrorCode::InvalidParams as i32,
            format!("bad params for {method}: {error}"),
        ),
    })
}
