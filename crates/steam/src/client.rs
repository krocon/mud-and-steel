//! The process-wide Steam client and the events its callbacks collect.

use crate::APP_ID;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use steamworks::{
    ChatMemberStateChange, Client, GameLobbyJoinRequested, GameRichPresenceJoinRequested,
    LobbyChatUpdate,
};

/// What the lobby screen has to react to.
#[derive(Clone, Debug)]
pub(crate) enum LobbyEvent {
    Created(Result<u64, String>),
    Joined(Result<u64, String>),
    List(Result<Vec<u64>, String>),
    /// A friend invited us, or we picked "Join game" in the friends list.
    JoinRequested(u64),
}

/// Collected by the callbacks, taken by [`OnlineLobby`](crate::OnlineLobby)
/// and [`SteamLink`](crate::SteamLink).
#[derive(Default)]
pub(crate) struct Shared {
    pub lobby_events: VecDeque<LobbyEvent>,
    /// Members that left a lobby: (lobby, user).
    pub left: Vec<(u64, u64)>,
    /// Users whose Networking Messages session failed or closed.
    pub failed: Vec<u64>,
    /// The lobby we are in; sessions are only accepted while in one.
    pub current_lobby: Option<u64>,
}

/// Handle to the initialized Steam client. Cheap to clone.
#[derive(Clone)]
pub struct Steam {
    client: Client,
    shared: Arc<Mutex<Shared>>,
}

/// The one client of this process; `None` until initialized successfully.
static STEAM: Mutex<Option<Steam>> = Mutex::new(None);

impl Steam {
    /// The Steam client, initialized on the first call. Fails (and may be
    /// retried later) if Steam is not running or not logged in.
    pub fn get() -> Result<Self, String> {
        let mut steam = STEAM.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(steam) = steam.as_ref() {
            return Ok(steam.clone());
        }
        let client = Client::init_app(APP_ID).map_err(|e| {
            let detail = match e {
                steamworks::SteamAPIInitError::FailedGeneric(m)
                | steamworks::SteamAPIInitError::NoSteamClient(m)
                | steamworks::SteamAPIInitError::VersionMismatch(m) => m,
            };
            format!(
                "Steam is not available ({}). Start Steam, log in and try again.",
                detail.trim().trim_end_matches('.')
            )
        })?;
        let initialized = Self::new(client);
        *steam = Some(initialized.clone());
        Ok(initialized)
    }

    /// Whether [`get`](Self::get) succeeded before (without trying again).
    pub fn is_initialized() -> bool {
        STEAM
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
    }

    fn new(client: Client) -> Self {
        let shared = Arc::new(Mutex::new(Shared::default()));
        // Start the relay network now: the first connection is faster.
        client.networking_utils().init_relay_network_access();

        let messages = client.networking_messages();
        let accept = shared.clone();
        messages.session_request_callback(move |request| {
            // Any peer may contact us while we are in a lobby; the lobby
            // protocol decides whether it becomes a player.
            if lock(&accept).current_lobby.is_some() {
                request.accept();
            } else {
                request.reject();
            }
        });
        let failed = shared.clone();
        messages.session_failed_callback(move |info| {
            if let Some(user) = info.identity_remote().and_then(|id| id.steam_id()) {
                lock(&failed).failed.push(user.raw());
            }
        });
        // The handles stay registered for the life of the process.
        let left = shared.clone();
        std::mem::forget(client.register_callback(move |update: LobbyChatUpdate| {
            if !matches!(update.member_state_change, ChatMemberStateChange::Entered) {
                lock(&left)
                    .left
                    .push((update.lobby.raw(), update.user_changed.raw()));
            }
        }));
        let invited = shared.clone();
        std::mem::forget(
            client.register_callback(move |request: GameLobbyJoinRequested| {
                lock(&invited)
                    .lobby_events
                    .push_back(LobbyEvent::JoinRequested(request.lobby_steam_id.raw()));
            }),
        );
        let presence = shared.clone();
        std::mem::forget(client.register_callback(
            move |request: GameRichPresenceJoinRequested| {
                if let Some(lobby) = crate::connect_string_lobby(&request.connect) {
                    lock(&presence)
                        .lobby_events
                        .push_back(LobbyEvent::JoinRequested(lobby));
                }
            },
        ));
        Self { client, shared }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Our Steam id.
    pub fn user_id(&self) -> u64 {
        self.client.user().steam_id().raw()
    }

    /// Our persona name.
    pub fn user_name(&self) -> String {
        self.client.friends().name()
    }

    /// The persona name of `user` (may be "[unknown]" until Steam knows it).
    pub fn name_of(&self, user: u64) -> String {
        self.client
            .friends()
            .get_friend(steamworks::SteamId::from_raw(user))
            .name()
    }

    /// Runs the pending Steam callbacks. Call every frame.
    pub fn run_callbacks(&self) {
        self.client.run_callbacks();
    }

    pub(crate) fn shared(&self) -> MutexGuard<'_, Shared> {
        lock(&self.shared)
    }

    /// A sender for call results, which run in `run_callbacks`.
    pub(crate) fn event_sink(&self) -> impl Fn(LobbyEvent) + Send + 'static {
        let shared = self.shared.clone();
        move |event| lock(&shared).lobby_events.push_back(event)
    }
}

fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
