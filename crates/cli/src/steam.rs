//! Headless online peers over Steam (prompt 026): `steam-host` and
//! `steam-join` play the demo scenario like `lan-host`/`lan-join`, with the
//! lobby found or joined through Steam. Needs a running, logged-in Steam
//! client per peer (two accounts, usually on two machines).

use crate::lan::{HASH_INTERVAL, MatchOptions, play_match};
use net::lobby::LobbyState;
use sim::scenario::demo_config;
use std::time::{Duration, Instant};
use steam::{OnlineLobby, OnlinePhase, Steam};

pub struct HostOptions {
    /// Listed for everyone, otherwise friends only.
    pub public: bool,
    pub seed: u64,
    pub input_delay: u32,
}

fn connect() -> Result<OnlineLobby, String> {
    let lobby = OnlineLobby::new(Steam::get()?);
    println!(
        "Steam: signed in as '{}' (app {})",
        lobby.user_name(),
        steam::APP_ID
    );
    Ok(lobby)
}

fn closed(phase: &OnlinePhase) -> Option<String> {
    match phase {
        OnlinePhase::Closed(reason) | OnlinePhase::InLobby(LobbyState::Closed(reason)) => {
            Some(reason.clone())
        }
        _ => None,
    }
}

/// Creates a lobby, waits for a second player who is ready, then plays.
pub fn host(options: &HostOptions, play: &MatchOptions) -> Result<i32, String> {
    let mut lobby = connect()?;
    lobby.host(2, options.public);
    let deadline = Instant::now() + play.timeout;
    let mut announced = false;
    loop {
        if Instant::now() > deadline {
            return Err("timed out waiting for a player".into());
        }
        lobby.poll();
        if let Some(reason) = closed(&lobby.phase()) {
            return Err(reason);
        }
        if !announced && let Some(code) = lobby.lobby_id() {
            announced = true;
            println!("lobby code {code}; waiting for a player (`just steam-join {code}`)");
        }
        if let Some(node) = lobby.node_mut()
            && node.can_start()
        {
            node.start(
                options.seed,
                demo_config(),
                options.input_delay,
                HASH_INTERVAL,
            )?;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    lobby.poll();
    play_match(lobby.take_started()?, play)
}

/// Joins the lobby `code` (or the first compatible one listed), gets ready
/// and plays.
pub fn join(code: Option<u64>, play: &MatchOptions) -> Result<i32, String> {
    let mut lobby = connect()?;
    let deadline = Instant::now() + play.timeout;
    match code {
        Some(code) => lobby.join(code),
        None => {
            println!("searching Steam lobbies of this game");
            let mut next_refresh = Instant::now();
            loop {
                if Instant::now() > deadline {
                    return Err("no lobby found".into());
                }
                if Instant::now() >= next_refresh {
                    lobby.refresh();
                    next_refresh = Instant::now() + Duration::from_secs(3);
                }
                lobby.poll();
                if let Some(found) = lobby.found().iter().find(|l| l.compatible).cloned() {
                    println!(
                        "found '{}' ({}/{}), lobby code {}",
                        found.name, found.players, found.max_players, found.id
                    );
                    lobby.join(found.id);
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    let mut ready = false;
    loop {
        if Instant::now() > deadline {
            return Err("timed out waiting for the start".into());
        }
        lobby.poll();
        let phase = lobby.phase();
        if let Some(reason) = closed(&phase) {
            return Err(reason);
        }
        match phase {
            OnlinePhase::InLobby(LobbyState::Lobby) if !ready => {
                if let Some(node) = lobby.node_mut() {
                    node.set_ready(true);
                    ready = true;
                    println!("in the lobby, ready");
                }
            }
            OnlinePhase::InLobby(LobbyState::Started) => break,
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    play_match(lobby.take_started()?, play)
}
