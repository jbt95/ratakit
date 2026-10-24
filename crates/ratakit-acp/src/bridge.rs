use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    CancelNotification, ClientCapabilities, ContentBlock, Implementation, InitializeRequest,
    NewSessionRequest, PromptRequest, PromptResponse, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
    SessionConfigOptionValue, SessionId as AcpSessionId, SessionNotification,
    SetSessionConfigOptionRequest, SetSessionModeRequest, TextContent,
};
use agent_client_protocol::{
    AcpAgent, AcpAgentConfig, Agent, Client, ConnectTo, ConnectionTo, on_receive_notification,
    on_receive_request,
};
use futures_util::stream::{FuturesUnordered, StreamExt};
use ratakit_agent::state::{
    AgentEvent, Effort, ModeId, ModelChoice, PermissionIntent, PermissionRequest, StopReason,
};
use tokio::sync::{mpsc, oneshot};

use crate::mapping::{
    EffortRoute, EventAccumulator, ModelRoute, ModelRoutes, acp_permission_request,
    map_config_options, map_session_update, map_stop_reason,
};

const EVENT_CHANNEL_CAPACITY: usize = 128;
const COMMAND_CHANNEL_CAPACITY: usize = 32;

/// A process command used to launch an ACP agent over its standard streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommand {
    /// Executable or command name.
    pub program: PathBuf,
    /// Arguments passed to the process without shell expansion.
    pub args: Vec<String>,
    /// Environment variables set for the child process.
    pub env: Vec<(String, String)>,
}

impl AgentCommand {
    /// Creates a command with no arguments or additional environment variables.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    /// Returns the standard Claude Code ACP launcher command.
    #[must_use]
    pub fn claude_code() -> Self {
        Self::new("npx").with_args(["-y", "@agentclientprotocol/claude-agent-acp"])
    }

    /// Returns the standard opencode ACP launcher command.
    #[must_use]
    pub fn opencode() -> Self {
        Self::new("opencode").with_args(["acp"])
    }

    /// Replaces the process arguments.
    #[must_use]
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Replaces the process environment additions.
    #[must_use]
    pub fn with_env<I, K, V>(mut self, env: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.env = env
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();
        self
    }

    fn into_agent(self) -> AcpAgent {
        AcpAgent::new(
            AcpAgentConfig::new(self.program)
                .args(self.args)
                .envs(self.env),
        )
    }
}

/// A ratakit session identifier returned by ACP `session/new`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Returns the opaque ACP session identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Identifier used by the bridge to correlate a permission prompt with its decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PermissionRequestId(pub u64);

/// Events emitted by an [`AcpBridge`].
#[derive(Debug, Clone, PartialEq)]
pub enum BridgeEvent {
    /// A protocol-neutral agent event to apply to the transcript state.
    Agent(AgentEvent),
    /// A permission request that must be answered with [`AcpBridgeHandle::respond_permission`].
    Permission(PermissionRequestId, PermissionRequest),
    /// The ACP transport ended. An error string is present for abnormal termination.
    Exited(Result<(), String>),
}

/// Errors returned by the ACP bridge API.
#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    /// The ACP SDK or agent returned an error.
    #[error("ACP protocol error: {0}")]
    Protocol(String),
    /// The command or event channel is closed.
    #[error("ACP bridge is closed")]
    Closed,
    /// A prompt, mode change, or model change was requested before creating a session.
    #[error("create an ACP session before sending session commands")]
    NoSession,
    /// The session working directory must be an absolute path.
    #[error("ACP session working directory must be absolute: {0}")]
    RelativeWorkingDirectory(PathBuf),
    /// A prompt is already in progress on this session.
    #[error("an ACP prompt is already in progress")]
    PromptInProgress,
    /// The requested model was not advertised by the agent.
    #[error("the ACP agent did not advertise model choice {provider}/{model}")]
    ModelUnavailable { provider: String, model: String },
    /// The requested effort was not advertised by the agent.
    #[error("the ACP agent did not advertise reasoning effort {0:?}")]
    EffortUnavailable(Effort),
    /// The requested permission prompt is no longer pending.
    #[error("permission request {0:?} is no longer pending")]
    UnknownPermission(PermissionRequestId),
}

/// A live ACP connection and its event stream.
pub struct AcpBridge {
    handle: AcpBridgeHandle,
    events: mpsc::Receiver<BridgeEvent>,
}

impl AcpBridge {
    /// Starts an ACP agent and initializes the protocol connection.
    ///
    /// The bridge does not advertise the ACP filesystem or terminal capabilities.
    /// The agent receives the requested working directory in `session/new`.
    ///
    /// # Errors
    ///
    /// Returns an error if the working directory is relative or the process cannot
    /// initialize an ACP connection.
    pub async fn spawn(command: AgentCommand, working_directory: &Path) -> Result<Self, AcpError> {
        if !working_directory.is_absolute() {
            return Err(AcpError::RelativeWorkingDirectory(
                working_directory.to_path_buf(),
            ));
        }

        Self::connect(command.into_agent(), working_directory.to_path_buf()).await
    }

    pub(super) async fn connect<T>(transport: T, cwd: PathBuf) -> Result<Self, AcpError>
    where
        T: ConnectTo<Client>,
    {
        let (command_tx, command_rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);
        let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let (ready_tx, ready_rx) = oneshot::channel();
        let shared = SharedState::default();
        let handle = AcpBridgeHandle {
            commands: command_tx,
            session_id: shared.session_id.clone(),
            permissions: shared.permissions.clone(),
        };

        tokio::spawn(async move {
            let result = run_client(
                transport,
                cwd,
                command_rx,
                event_tx.clone(),
                ready_tx,
                shared,
            )
            .await;
            let _ = event_tx.send(BridgeEvent::Exited(result.clone())).await;
        });

        match ready_rx.await {
            Ok(Ok(())) => Ok(Self {
                handle,
                events: event_rx,
            }),
            Ok(Err(error)) => Err(AcpError::Protocol(error)),
            Err(_) => Err(AcpError::Protocol(
                "connection ended before ACP initialization completed".into(),
            )),
        }
    }

    /// Returns a cloneable handle for sending commands while polling bridge events.
    #[must_use]
    pub fn handle(&self) -> AcpBridgeHandle {
        self.handle.clone()
    }

    /// Creates a new ACP session in the working directory supplied to [`Self::spawn`].
    ///
    /// # Errors
    ///
    /// Returns [`AcpError::Closed`] if the bridge has stopped or
    /// [`AcpError::Protocol`] if the agent rejects the session request.
    pub async fn new_session(&self) -> Result<SessionId, AcpError> {
        self.handle.new_session().await
    }

    /// Receives translated agent events, permission prompts, and process termination.
    pub fn events(&mut self) -> &mut mpsc::Receiver<BridgeEvent> {
        &mut self.events
    }
}

/// Cloneable command handle for an [`AcpBridge`].
#[derive(Clone)]
pub struct AcpBridgeHandle {
    commands: mpsc::Sender<Command>,
    session_id: Arc<Mutex<Option<SessionId>>>,
    permissions: Arc<Mutex<HashMap<PermissionRequestId, oneshot::Sender<PermissionIntent>>>>,
}

impl AcpBridgeHandle {
    /// Creates a new session and stores its ID for subsequent commands.
    ///
    /// # Errors
    ///
    /// Returns [`AcpError::Closed`] if the connection has ended, or
    /// [`AcpError::Protocol`] if the agent rejects the session request.
    pub async fn new_session(&self) -> Result<SessionId, AcpError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::NewSession(reply_tx))
            .await
            .map_err(|_| AcpError::Closed)?;
        receive_reply(reply_rx).await
    }

    /// Sends a user prompt to the current session and waits for its stop reason.
    ///
    /// Mention paths are included as plain-text references. The bridge does not
    /// expose filesystem capabilities to the agent.
    ///
    /// # Errors
    ///
    /// Returns [`AcpError::NoSession`] before session creation,
    /// [`AcpError::PromptInProgress`] when another prompt is running, or an ACP
    /// protocol error if the agent rejects the request or the connection closes.
    pub async fn prompt(
        &self,
        text: impl Into<String>,
        mentions: &[PathBuf],
    ) -> Result<StopReason, AcpError> {
        let session_id = self.current_session()?;
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::Prompt {
                session_id,
                text: text.into(),
                mentions: mentions.to_vec(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| AcpError::Closed)?;
        receive_reply(reply_rx).await
    }

    /// Cancels the current prompt and resolves outstanding permission requests as cancelled.
    ///
    /// # Errors
    ///
    /// Returns [`AcpError::NoSession`] before session creation or a protocol error
    /// if cancellation cannot be sent.
    pub async fn cancel(&self) -> Result<(), AcpError> {
        let session_id = self.current_session()?;
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::Cancel {
                session_id,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AcpError::Closed)?;
        receive_reply(reply_rx).await
    }

    /// Changes the current session mode.
    ///
    /// # Errors
    ///
    /// Returns [`AcpError::NoSession`] before session creation or a protocol error
    /// if the agent rejects the requested mode.
    pub async fn set_mode(&self, mode: &ModeId) -> Result<(), AcpError> {
        let session_id = self.current_session()?;
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::SetMode {
                session_id,
                mode: mode.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| AcpError::Closed)?;
        receive_reply(reply_rx).await
    }

    /// Changes the current model and, when supplied, its reasoning effort.
    ///
    /// # Errors
    ///
    /// Returns [`AcpError::NoSession`] before session creation,
    /// [`AcpError::ModelUnavailable`] or [`AcpError::EffortUnavailable`] when the
    /// agent did not advertise the selection, or a protocol error on rejection.
    pub async fn set_model(&self, choice: &ModelChoice) -> Result<(), AcpError> {
        let session_id = self.current_session()?;
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::SetModel {
                session_id,
                choice: choice.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| AcpError::Closed)?;
        receive_reply(reply_rx).await
    }

    /// Sends a user's decision for a pending ACP permission request.
    ///
    /// # Errors
    ///
    /// Returns [`AcpError::UnknownPermission`] if the request is no longer pending,
    /// or [`AcpError::Closed`] if the permission state lock is poisoned.
    pub fn respond_permission(
        &self,
        id: PermissionRequestId,
        choice: PermissionIntent,
    ) -> Result<(), AcpError> {
        let sender = self
            .permissions
            .lock()
            .map_err(|_| AcpError::Closed)?
            .remove(&id)
            .ok_or(AcpError::UnknownPermission(id))?;
        sender
            .send(choice)
            .map_err(|_| AcpError::UnknownPermission(id))
    }

    fn current_session(&self) -> Result<AcpSessionId, AcpError> {
        self.session_id
            .lock()
            .map_err(|_| AcpError::Closed)?
            .as_ref()
            .map(|id| AcpSessionId::new(id.0.clone()))
            .ok_or(AcpError::NoSession)
    }
}

#[derive(Default)]
struct SharedState {
    session_id: Arc<Mutex<Option<SessionId>>>,
    permissions: Arc<Mutex<HashMap<PermissionRequestId, oneshot::Sender<PermissionIntent>>>>,
    model_routes: Arc<Mutex<ModelRoutes>>,
    next_permission_id: Arc<AtomicU64>,
    accumulator: Arc<Mutex<EventAccumulator>>,
}

enum Command {
    NewSession(oneshot::Sender<Result<SessionId, String>>),
    Prompt {
        session_id: AcpSessionId,
        text: String,
        mentions: Vec<PathBuf>,
        reply: oneshot::Sender<Result<StopReason, String>>,
    },
    Cancel {
        session_id: AcpSessionId,
        reply: oneshot::Sender<Result<(), String>>,
    },
    SetMode {
        session_id: AcpSessionId,
        mode: ModeId,
        reply: oneshot::Sender<Result<(), String>>,
    },
    SetModel {
        session_id: AcpSessionId,
        choice: ModelChoice,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

type PromptReply = oneshot::Sender<Result<StopReason, String>>;
type PendingPrompt = Pin<
    Box<
        dyn Future<
                Output = (
                    PromptReply,
                    Result<PromptResponse, agent_client_protocol::Error>,
                ),
            > + Send,
    >,
>;

async fn receive_reply<T>(reply: oneshot::Receiver<Result<T, String>>) -> Result<T, AcpError> {
    reply
        .await
        .map_err(|_| AcpError::Closed)?
        .map_err(AcpError::Protocol)
}

// This is the single connection-driving select loop; command-specific work is
// intentionally kept together so request ordering and prompt concurrency stay visible.
#[allow(clippy::too_many_lines)]
async fn run_client<T>(
    transport: T,
    cwd: PathBuf,
    mut commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<BridgeEvent>,
    ready: oneshot::Sender<Result<(), String>>,
    shared: SharedState,
) -> Result<(), String>
where
    T: ConnectTo<Client>,
{
    let notification_events = events.clone();
    let notification_accumulator = shared.accumulator.clone();
    let notification_routes = shared.model_routes.clone();

    let permission_events = events.clone();
    let permission_senders = shared.permissions.clone();
    let next_permission_id = shared.next_permission_id.clone();

    Client
        .builder()
        .name("ratakit")
        .on_receive_notification(
            async move |notification: SessionNotification, _connection| {
                let mapped = {
                    let mut accumulator = notification_accumulator
                        .lock()
                        .map_err(|_| agent_client_protocol::Error::internal_error())?;
                    map_session_update(
                        notification.update,
                        &mut accumulator,
                        &notification_routes,
                    )
                };
                for event in mapped {
                    if notification_events
                        .send(BridgeEvent::Agent(event))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(())
            },
            on_receive_notification!(),
        )
        .on_receive_request(
            async move |request: RequestPermissionRequest, responder, _connection| {
                let id = PermissionRequestId(
                    next_permission_id.fetch_add(1, Ordering::Relaxed).saturating_add(1),
                );
                let (decision_tx, decision_rx) = oneshot::channel();
                if let Ok(mut pending) = permission_senders.lock() {
                    pending.insert(id, decision_tx);
                }

                let request_view = acp_permission_request(&request);
                if permission_events
                    .send(BridgeEvent::Permission(id, request_view))
                    .await
                    .is_err()
                {
                    if let Ok(mut pending) = permission_senders.lock() {
                        pending.remove(&id);
                    }
                    return responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    ));
                }

                let decision = decision_rx.await.unwrap_or(PermissionIntent::Dismissed);
                if let Ok(mut pending) = permission_senders.lock() {
                    pending.remove(&id);
                }
                responder.respond(permission_response(&request, decision))
            },
            on_receive_request!(),
        )
        .connect_with(transport, async move |connection: ConnectionTo<Agent>| {
            let initialize = connection
                .send_request(
                    InitializeRequest::new(ProtocolVersion::V1)
                        .client_capabilities(ClientCapabilities::default())
                        .client_info(Implementation::new("ratakit", env!("CARGO_PKG_VERSION"))),
                )
                .block_task()
                .await;
            if let Err(error) = initialize {
                let message = error.to_string();
                let _ = ready.send(Err(message.clone()));
                return Err(error);
            }
            let _ = ready.send(Ok(()));

            let mut active_session: Option<AcpSessionId> = None;
            let mut pending_prompts = FuturesUnordered::<PendingPrompt>::new();

            loop {
                tokio::select! {
                    command = commands.recv() => {
                        let Some(command) = command else {
                            break;
                        };
                        match command {
                            Command::NewSession(reply) => {
                                if !pending_prompts.is_empty() {
                                    let _ = reply.send(Err("cannot replace a session while a prompt is in progress".into()));
                                    continue;
                                }
                                let response = connection
                                    .send_request(NewSessionRequest::new(cwd.clone()))
                                    .block_task()
                                    .await;
                                match response {
                                    Ok(response) => {
                                        active_session = Some(response.session_id.clone());
                                        if let Ok(mut session) = shared.session_id.lock() {
                                            *session = Some(SessionId(response.session_id.to_string()));
                                        }
                                        if let Some(modes) = response.modes {
                                            let _ = events.send(BridgeEvent::Agent(
                                                AgentEvent::ModeChanged(ModeId::new(
                                                    modes.current_mode_id.to_string(),
                                                )),
                                            )).await;
                                        }
                                        if let Some(config_options) = response.config_options
                                            && let Some(choices) = map_config_options(
                                                &config_options,
                                                &shared.model_routes,
                                            )
                                        {
                                            let _ = events.send(BridgeEvent::Agent(
                                                AgentEvent::ModelChoicesAvailable(choices),
                                            )).await;
                                        }
                                        let _ = reply.send(Ok(SessionId(response.session_id.to_string())));
                                    }
                                    Err(error) => {
                                        let _ = reply.send(Err(error.to_string()));
                                    }
                                }
                            }
                            Command::Prompt { session_id, text, mentions, reply } => {
                                if !pending_prompts.is_empty() {
                                    let _ = reply.send(Err(AcpError::PromptInProgress.to_string()));
                                    continue;
                                }
                                if active_session.as_ref() != Some(&session_id) {
                                    let _ = reply.send(Err(AcpError::NoSession.to_string()));
                                    continue;
                                }
                                let prompt_text = prompt_with_mentions(text, &mentions);
                                let request = connection.send_request(PromptRequest::new(
                                    session_id,
                                    vec![ContentBlock::Text(TextContent::new(prompt_text))],
                                ));
                                pending_prompts.push(Box::pin(async move {
                                    (reply, request.block_task().await)
                                }));
                            }
                            Command::Cancel { session_id, reply } => {
                                if active_session.as_ref() != Some(&session_id) {
                                    let _ = reply.send(Err(AcpError::NoSession.to_string()));
                                    continue;
                                }
                                let result = connection
                                    .send_notification(CancelNotification::new(session_id))
                                    .map_err(|error| error.to_string());
                                cancel_pending_permissions(&shared.permissions);
                                let _ = reply.send(result);
                            }
                            Command::SetMode { session_id, mode, reply } => {
                                if active_session.as_ref() != Some(&session_id) {
                                    let _ = reply.send(Err(AcpError::NoSession.to_string()));
                                    continue;
                                }
                                let result = connection
                                    .send_request(SetSessionModeRequest::new(session_id, mode.as_str().to_owned()))
                                    .block_task()
                                    .await
                                    .map(|_| ())
                                    .map_err(|error| error.to_string());
                                let _ = reply.send(result);
                            }
                            Command::SetModel { session_id, choice, reply } => {
                                if active_session.as_ref() != Some(&session_id) {
                                    let _ = reply.send(Err(AcpError::NoSession.to_string()));
                                    continue;
                                }
                                let (model_route, effort_route) = match resolve_model_routes(
                                    &shared.model_routes,
                                    &choice,
                                ) {
                                    Ok(routes) => routes,
                                    Err(error) => {
                                        let _ = reply.send(Err(error));
                                        continue;
                                    }
                                };

                                let result = async {
                                    connection
                                        .send_request(SetSessionConfigOptionRequest::new(
                                            session_id.clone(),
                                            model_route.config_id,
                                            SessionConfigOptionValue::value_id(model_route.value_id),
                                        ))
                                        .block_task()
                                        .await?;
                                    if let Some(route) = effort_route {
                                        connection
                                            .send_request(SetSessionConfigOptionRequest::new(
                                                session_id,
                                                route.config_id,
                                                SessionConfigOptionValue::value_id(route.value_id),
                                            ))
                                            .block_task()
                                            .await?;
                                    }
                                    Ok::<(), agent_client_protocol::Error>(())
                                }
                                .await
                                .map_err(|error| error.to_string());
                                let _ = reply.send(result);
                            }
                        }
                    }
                    Some((reply, result)) = pending_prompts.next(), if !pending_prompts.is_empty() => {
                        match result {
                            Ok(response) => {
                                let reason = map_stop_reason(response.stop_reason);
                                emit_agent_events(&events, finish_turn(&shared.accumulator, reason)).await;
                                let _ = reply.send(Ok(reason));
                            }
                            Err(error) => {
                                let _ = events.send(BridgeEvent::Agent(AgentEvent::Error(error.to_string()))).await;
                                emit_agent_events(&events, finish_turn(&shared.accumulator, StopReason::Error)).await;
                                let _ = reply.send(Err(error.to_string()));
                            }
                        }
                    }
                }
            }
            Ok(())
        })
        .await
        .map_err(|error| error.to_string())
}

async fn emit_agent_events(events: &mpsc::Sender<BridgeEvent>, agent_events: Vec<AgentEvent>) {
    for event in agent_events {
        if events.send(BridgeEvent::Agent(event)).await.is_err() {
            break;
        }
    }
}

fn resolve_model_routes(
    routes: &Mutex<ModelRoutes>,
    choice: &ModelChoice,
) -> Result<(ModelRoute, Option<EffortRoute>), String> {
    let routes = routes.lock().map_err(|_| {
        agent_client_protocol::util::internal_error("model route lock was poisoned").to_string()
    })?;
    let model = routes.model_route(choice).ok_or_else(|| {
        AcpError::ModelUnavailable {
            provider: choice.provider.clone(),
            model: choice.model.clone(),
        }
        .to_string()
    })?;
    let effort = choice
        .effort
        .map(|effort| {
            routes
                .effort_route(effort)
                .ok_or_else(|| AcpError::EffortUnavailable(effort).to_string())
        })
        .transpose()?;
    Ok((model, effort))
}

fn finish_turn(accumulator: &Arc<Mutex<EventAccumulator>>, reason: StopReason) -> Vec<AgentEvent> {
    accumulator.lock().map_or_else(
        |_| vec![AgentEvent::TurnEnded { reason }],
        |mut state| state.finish_turn(reason),
    )
}

pub(super) fn prompt_with_mentions(mut text: String, mentions: &[PathBuf]) -> String {
    if !mentions.is_empty() {
        text.push_str("\n\nReferenced files:\n");
        for path in mentions {
            text.push_str("- @");
            text.push_str(&path.to_string_lossy());
            text.push('\n');
        }
    }
    text
}

pub(super) fn permission_response(
    request: &RequestPermissionRequest,
    decision: PermissionIntent,
) -> RequestPermissionResponse {
    let outcome = match decision {
        PermissionIntent::Chose(choice) => request
            .options
            .iter()
            .find(|option| option.option_id.to_string() == choice.0)
            .map_or(RequestPermissionOutcome::Cancelled, |option| {
                RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                    option.option_id.clone(),
                ))
            }),
        PermissionIntent::Dismissed => RequestPermissionOutcome::Cancelled,
    };
    RequestPermissionResponse::new(outcome)
}

fn cancel_pending_permissions(
    permissions: &Arc<Mutex<HashMap<PermissionRequestId, oneshot::Sender<PermissionIntent>>>>,
) {
    if let Ok(mut permissions) = permissions.lock() {
        for (_, sender) in permissions.drain() {
            let _ = sender.send(PermissionIntent::Dismissed);
        }
    }
}
