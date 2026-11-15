extends RefCounted
## Hands a started LAN game from the lobby scene to `main.tscn`.
##
## Static, so it survives the scene change. `main.gd` passes `lobby` to
## `SimHost.start_lan` and clears it. Used through `preload`, not a global
## class name, so it works without an editor import.

## The started `LanLobby`, or null for single player.
static var lobby: LanLobby = null
## `--lan-self-test`: play automatically and quit with exit code 0 or 1.
static var self_test := false
