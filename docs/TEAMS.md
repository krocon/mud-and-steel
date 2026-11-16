# Teams, computer slots and game modes

Prompt 037. Two or three players per side (2 against 2, 3 against 3),
humans and computer players mixed, in single player, on the LAN and over
Steam. Decisions marked **[provisional]** still need the user's
confirmation; the reasons are in `prompts/done/037-teams-ki-slots.md`.

## Rules (simulation)

`SimConfig::teams` (`crates/sim/src/teams.rs`) lists `(player, team)` and
whether allies share their vision. A player without an entry is on a team
of their own, so the default is every player against every other, as
before. `Teams::new` turns "everybody on a different team" into that
default, so a 1-against-1 from the lobby is bit for bit the old match.

| Rule | Where |
|---|---|
| Allies are no automatic targets (direct and indirect fire) | `sim/combat.rs` `choose_target`, `sim/indirect.rs` |
| An `Attack` order on an ally is ignored | `sim.rs` `apply` |
| Units do not fire through allies; mortars do not fire near them; rounds near allies do not suppress them | `friend_in_line`, `friend_near`, `near_miss` |
| Allies always see each other's units | `sim/vision.rs` (`allies` mask per player) |
| Shared vision (option, default on **[provisional]**): a player sees what the allies see; the fog shows the best state of the team | `sim/vision.rs` (`shares` mask), `Sim::vision_states` |
| Allies do not contest each other's supply points; a point held by an ally counts as held | `sim/economy.rs` |
| Flak does not fire at allied planes; allies see each other's planes and, with shared vision, the planes an ally spots (prompt 033 merged) | `sim/air.rs` |
| Nobody commands an ally's units | unchanged (`owned_indices`) |
| A match is decided once every undefeated player is an ally of the others; `Outcome::winner` is the lowest undefeated player of the winning team, `Sim::winners()` the whole team | `outcome.rs` |
| A surrender hands nothing over: the units stay where they are with their fire mode **[provisional]** | `outcome.rs` |

Ground attacks and fragments still hurt everybody: they are physics, and
the AI keeps barrages away from allies.

**State and hash.** The teams are stored in `Sim` (saved games of prompt
038 include them; `net::snapshot::rules_hash` changed with the layout) and
enter the state hash only when there are teams: every free-for-all hashes
as before, the golden hash is unchanged.

**Ping.** `Command::Ping { target }` (appended to the enum) has no effect
in the simulation. It travels through the lockstep session and the replay
like every command; the bridge (`team_funcs::collect_pings`) hands the pings
of the local player and its allies to Godot.

## Scenarios and maps

`scenario::parse_scenario_for_slots(text, slots, shared_vision, …)` builds
a match for the players in `slots` (player, team, faction). A player the
scenario has no groups for gets the groups of the scenario player whose
start is nearest to theirs, moved to their own start; the map must have a
start for every player. Demo battle and base building work for 2 to 6
players without new scenario files.

Maps: `[[start]]` takes an optional `team` (the lobby's suggestion; not
part of the map hash, like the starts). `cli gen-map --players 4|6` writes
two teams on the start meadow, south (even players) against north (odd
players), every start mirrored through the center from its opponent's, and
two or three more supply points on the meadow (kept free of objects,
equally far from both teams).

| Map | Size | Starts | Supply points |
|---|---|---|---|
| `test_small` | 500 m | 2 (unchanged: golden hash) | 3 |
| `twin_meadow` (new, `gen-map … 1000 21 --players 4`) | 1 km | 4: 0 + 2 against 1 + 3 | 5 |
| `test_large` (regenerated with `--players 6`, rasters and objects unchanged) | 4 km | 6: 0 + 2 + 4 against 1 + 3 + 5 | 6 |

The map editor places starts for six players and sets a team per start
(new starts get `player % 2`; the panel changes or removes it); the markers
take the team's color.

## Lobby (single player, LAN, Steam)

Slot *i* is player *i* (start position and colour stay stable). A slot is
**open**, **human**, **computer** (with difficulty) or **closed**, with a
team and a faction (empty: the scenario's). The number of slots is the
map's number of starts (at most 6).

- **Single player** (setup screen): *Players* 1 vs 1 (the old options:
  your side, enemy, opponent) or 2 vs 2 / 3 vs 3 on maps with enough
  starts: a slot list with you in slot 0 and computers everywhere else
  (difficulty of *Opponent*, teams from the map, your faction for your
  team and the enemy's for the other), each slot changeable;
  *Allies share what they see*. `SimHost.start_slots(seed, map_dir,
  slots, shared_vision)` starts it; `SimHost.mode` gives the mode.
- **LAN / Steam**: the host picks map, mode (battle, base building) and
  shared vision before hosting; in the lobby the host sets every slot, a
  client its own team and faction. `net::lobby::LobbySetup` travels with
  `Control::Setup`; a client asks with `Control::Choose`. A joining human
  takes the first open slot. The host can start once every human is ready
  and at least two players on two teams take part.
- **Computer players** run on the host (`LockstepSession::host_player`,
  as prepared in prompt 024). `StartInfo.players` lists every player of the
  session, `StartInfo.computers` the computers with their difficulty; the
  other peers cannot tell them from humans.
- `net::lobby::PROTOCOL_VERSION` is 3, the replay format 7.

## Computer players

`ai::PlayerView` separates allied units (`allies`) from enemies and lists
the enemy players with whether they are defeated. The AI heads for the
nearest home (base or start) of an undefeated enemy, then the next one.
"Helping" is kept simple: with shared vision the enemies an ally fights
are in the AI's sightings, so its groups go there. Barrages keep away from
allies. End game of base matches: at 85 % of the unit limit the AI no
longer waits for waves, and with nothing left to search it goes for supply
points enemies hold before random points. Details and numbers:
[AI.md](AI.md#teams-prompt-037).

## Presentation

- Colours **[provisional]**: own units as before (no tint), allies blue,
  enemies red, in the world (tint), on the minimap, the health bars and
  the supply flags. Allies are no targets for a right click.
- **Ping:** `K` *(rebindable, `ping_allies`)* marks the ground under the
  mouse, Alt + left click marks the minimap point; the allies see a ring on
  the minimap and a message in the event feed.
- The result screen counts the team: victory when the own team won,
  "Your team was defeated" when an ally fought on after one's own defeat.

## Tests

| Test | What |
|---|---|
| `crates/sim/tests/teams.rs` | allies do not fire at each other (also not on order), allies see each other, shared vision, team victory with a surrender, 1 vs 1 from slots equals the old scenario, 2 vs 2 on `twin_meadow` in both modes, a missing start is refused |
| `crates/sim/src/outcome.rs`, `teams.rs` | team standings, `Teams::new` |
| `crates/net/tests/lobby_slots.rs` | computer, closed and open slots, teams, factions, a client's own choice, full lobby |
| `crates/net/tests/snapshot_teams.rs` | a saved 2 vs 2 continues identically |
| `crates/ai/tests/teams.rs` | the view knows allies, no AI ever attacks an ally, every AI builds in a 2 vs 2 base match |
| `crates/ai/tests/lobby_teams.rs` | two humans and two computers (hosted by the host) through the lobby over memory links: in sync, verified every tick |
| `crates/editor/tests/start_teams.rs` | start teams saved, loaded, undone |
| `just lan-selftest` | two Godot instances: the two humans against two computers the host runs on `twin_meadow` (hold fire); the client surrenders and the match goes on, then the host, both see the computers' team win (`LAN_COMPUTERS=0`: the old 1 vs 1) |
| `just selftest` | the flow self-test starts a 2 vs 2 from the setup screen: four players, relations, allied units drawn, a ping reaches the minimap, a surrender does not end the match while the ally stands |
| `just editor-selftest` | a third start with its team, changed from the panel |
| `just ai-tournament --teams 2v2` | team tournaments (A plays one team, B the other) |

## Open

Team chat, giving resources or units to allies, reconnect, spectators,
more than six players, a host changing the map after hosting (leave and
host again), tuning the AI for the larger maps (see AI.md), a real Steam
test with more than two accounts.
