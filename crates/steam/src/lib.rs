//! Online multiplayer over Steam (prompt 026), with `steamworks-rs`.
//!
//! The lobby protocol and the lockstep transport are the same as on the LAN
//! ([`net::lobby::LobbyNode`]); this crate only adds what is Steam-specific:
//!
//! - [`Steam`]: the process-wide Steam client, initialized on first use with
//!   [`APP_ID`]. Fails cleanly when the Steam client is not running.
//! - [`SteamLink`]: the [`net::lobby::Link`] over Steam Networking Messages
//!   (relayed, NAT traversal included), addressed by Steam id.
//! - [`OnlineLobby`]: create, find and join Steam lobbies, invite friends,
//!   rich presence; runs a `LobbyNode<SteamLink>` once inside a lobby.
//!
//! Nothing here runs unless a player opens the online menu, so single
//! player, LAN and the tests never touch Steam. See `docs/ONLINE.md`.

mod client;
mod link;
mod lobby;

pub use client::Steam;
pub use link::{SteamLink, SteamNode};
pub use lobby::{FoundLobby, OnlineLobby, OnlinePhase};

/// The Steam app id: the only place it is configured. 480 is Valve's public
/// test app ("Spacewar") until Mud and Steel has its own.
pub const APP_ID: u32 = 480;

/// Lobby data key and value that mark our lobbies. On the shared test app
/// every Steamworks developer creates lobbies; the lobby list filters by it.
pub const LOBBY_GAME_KEY: &str = "game";
pub const LOBBY_GAME: &str = "mud-and-steel";

/// The launch option Steam passes when a friend joins through the friends
/// list while the game is not running: `+connect_lobby <lobby id>`.
pub const CONNECT_LOBBY_ARG: &str = "+connect_lobby";

/// The lobby id after [`CONNECT_LOBBY_ARG`] in `args`, if any.
pub fn connect_lobby_arg<S: AsRef<str>>(args: &[S]) -> Option<u64> {
    let index = args.iter().position(|a| a.as_ref() == CONNECT_LOBBY_ARG)?;
    args.get(index + 1)?.as_ref().trim().parse().ok()
}

/// The lobby id in a rich presence connect string (`+connect_lobby <id>`).
pub fn connect_string_lobby(connect: &str) -> Option<u64> {
    let args: Vec<&str> = connect.split_whitespace().collect();
    connect_lobby_arg(&args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_lobby_in_the_launch_options() {
        let args = ["game", "--", CONNECT_LOBBY_ARG, "109775241234567890"];
        assert_eq!(connect_lobby_arg(&args), Some(109_775_241_234_567_890));
        assert_eq!(connect_lobby_arg(&["game"]), None);
        assert_eq!(connect_lobby_arg(&[CONNECT_LOBBY_ARG]), None);
        assert_eq!(connect_lobby_arg(&[CONNECT_LOBBY_ARG, "x"]), None);
    }

    #[test]
    fn finds_the_lobby_in_a_connect_string() {
        let connect = format!("{CONNECT_LOBBY_ARG} 42");
        assert_eq!(connect_string_lobby(&connect), Some(42));
        assert_eq!(connect_string_lobby("+connect 1.2.3.4"), None);
    }
}
