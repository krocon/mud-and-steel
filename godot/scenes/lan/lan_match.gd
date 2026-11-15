extends RefCounted
## Hands a started network game (LAN, or online over Steam since prompt 026)
## from the lobby scene to `main.tscn`.
##
## Static, so it survives the scene change. `main.gd` passes `lobby` to
## `SimHost.start_lan` and clears it. Used through `preload`, not a global
## class name, so it works without an editor import.

## The started `LanLobby` or `SteamLobby`, or null for single player.
static var lobby: RefCounted = null
## Shown in the status line: "LAN" or "Online".
static var kind := "LAN"
## `--lan-self-test` / `--steam-self-test`: play automatically and quit with exit code 0 or 1.
static var self_test := false
