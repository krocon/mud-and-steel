//! The computer opponent in single player (prompt 024): every player of the
//! match except the local one is an `ai::Ai`, hosted in the local lockstep
//! session. Its commands go through the session like the player's own (and
//! into the replay); it only sees its player's fog-of-war view.

use crate::{Game, SimHost};
use godot::prelude::*;
use net::NetError;
use sim::{PlayerId, SimConfig};

/// The difficulty `SimHost.ai_opponent` names, `None` when empty.
fn difficulty(host: &SimHost) -> Option<ai::Difficulty> {
    let name = host.ai_opponent.to_string();
    if name.is_empty() {
        return None;
    }
    match name.parse() {
        Ok(d) => Some(d),
        Err(e) => {
            godot_error!("{e}; starting without a computer opponent");
            None
        }
    }
}

/// Players of the single-player session: only the local one, or everyone
/// when computer players take part (the local one and the computers of
/// `start_slots`, prompt 037).
pub(crate) fn session_players(
    host: &SimHost,
    config: &SimConfig,
    local: PlayerId,
) -> Vec<PlayerId> {
    if !host.computers.is_empty() {
        let mut players: Vec<PlayerId> = host.computers.iter().map(|&(p, _)| p).collect();
        players.push(local);
        players.sort_unstable();
        players.dedup();
        return players;
    }
    if difficulty(host).is_none() {
        return vec![local];
    }
    let mut players = config.players();
    if !players.contains(&local) {
        players.push(local);
        players.sort_unstable();
    }
    players
}

/// Hosts a computer player for every player but the local one (or the
/// computers of `start_slots`). Returns `true` (the match started either
/// way).
pub(crate) fn attach(host: &mut SimHost, seed: u64) -> bool {
    if !host.computers.is_empty() {
        let computers = std::mem::take(&mut host.computers);
        if let Some(game) = host.game.as_mut() {
            host_computers(game, &computers, seed);
            game.networked = false;
        }
        return true;
    }
    let Some(difficulty) = difficulty(host) else {
        return true;
    };
    let Some(game) = host.game.as_mut() else {
        return true;
    };
    let local = game.session.local_player();
    for player in game.config.players() {
        if player == local {
            continue;
        }
        game.session.host_player(player);
        game.ais.push(ai::Ai::new(
            player,
            difficulty,
            &game.config,
            game.map(),
            seed,
        ));
    }
    // Computer players run here: nothing goes over a network.
    game.networked = false;
    godot_print!(
        "computer opponent: {difficulty} for players {:?}",
        game.ais.iter().map(|a| a.player().0).collect::<Vec<_>>()
    );
    true
}

/// Hosts the given computer players in the session of `game` (single
/// player with slots, or the host of a network match, prompt 037).
pub(crate) fn host_computers(game: &mut Game, computers: &[(PlayerId, ai::Difficulty)], seed: u64) {
    for &(player, difficulty) in computers {
        if player == game.session.local_player() {
            continue;
        }
        game.session.host_player(player);
        game.ais.push(ai::Ai::new(
            player,
            difficulty,
            &game.config,
            game.map(),
            seed,
        ));
    }
    if !game.ais.is_empty() {
        godot_print!(
            "computer players: {:?}",
            game.ais
                .iter()
                .map(|a| format!("{} {}", a.player().0, a.difficulty()))
                .collect::<Vec<_>>()
        );
    }
}

/// Sends the frames of the hosted computer players for the tick of the
/// local frame just submitted. They think about the state after the last
/// simulated tick, like the player whose commands are in the local frame.
pub(crate) fn submit(game: &mut Game) -> Result<(), NetError> {
    for ai in &mut game.ais {
        let view = ai::PlayerView::new(&game.sim, ai.player());
        let commands = ai.think(&view);
        game.session
            .submit_hosted(ai.player(), commands, &mut game.transport)?;
    }
    Ok(())
}

#[godot_api(secondary)]
impl SimHost {
    /// Difficulty of the computer players of the running match (`""`
    /// without), and how many commands-deciding ticks they had.
    #[func]
    fn ai_status(&self) -> VarDictionary {
        let Some(game) = &self.game else {
            return vdict! { "difficulty" => "", "players" => 0i64, "decisions" => 0i64 };
        };
        vdict! {
            "difficulty" => game.ais.first().map_or(String::new(), |a| a.difficulty().to_string()),
            "players" => game.ais.len() as i64,
            "decisions" => game.ais.iter().map(|a| a.decisions()).sum::<u64>() as i64,
        }
    }
}
