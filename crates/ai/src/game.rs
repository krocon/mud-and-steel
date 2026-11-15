//! Matches between computer players, headless: the tournament of the CLI
//! and the tests. The AIs send their commands through a lockstep session
//! (one AI as the local player, the others hosted, prompt 024) exactly like
//! the game does, and the match is recorded as a replay.

use crate::{Ai, Difficulty, PlayerView};
use net::{LocalTransport, LockstepConfig, LockstepSession, Replay, ReplayRecorder};
use sim::{Command, Map, PlayerId, Sim, SimConfig, StableHasher};
use std::sync::Arc;

/// Input delay of the AIs' commands, as in single player.
pub const INPUT_DELAY: u32 = 1;

/// Result of one match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameResult {
    pub seed: u64,
    /// `None` for a draw or a match that ran out of time.
    pub winner: Option<PlayerId>,
    /// Tick in which the match was decided; `None` if it ran out of time.
    pub decided_tick: Option<u64>,
    /// Ticks simulated.
    pub ticks: u64,
    /// Units not destroyed per player at the end, by player id.
    pub alive: Vec<(PlayerId, usize)>,
    /// `Sim::state_hash` at the end.
    pub end_hash: u64,
    /// Commands the AIs gave, per player.
    pub commands: Vec<(PlayerId, usize)>,
}

impl GameResult {
    /// Feeds the result into `h` (for a tournament hash).
    pub fn hash_into(&self, h: &mut StableHasher) {
        h.write_u64(self.seed);
        h.write_u32(self.winner.map_or(u32::MAX, |p| u32::from(p.0)));
        h.write_u64(self.decided_tick.unwrap_or(u64::MAX));
        h.write_u64(self.ticks);
        for &(p, n) in &self.alive {
            h.write_u8(p.0);
            h.write_u64(n as u64);
        }
        h.write_u64(self.end_hash);
    }
}

/// A match in which every player is a computer player.
pub struct Game {
    sim: Sim,
    session: LockstepSession,
    transport: LocalTransport,
    recorder: ReplayRecorder,
    /// Sorted by player; the first one is the session's local player.
    ais: Vec<Ai>,
    commands: Vec<usize>,
    seed: u64,
}

impl Game {
    /// Starts `config` on `map` with `seed`. `players` lists the difficulty
    /// of every player of the config; players not listed are passive.
    pub fn new(
        config: &SimConfig,
        map: Arc<Map>,
        seed: u64,
        players: &[(PlayerId, Difficulty)],
    ) -> Result<Self, String> {
        let sim = Sim::new(config, map.clone(), seed).map_err(|e| e.to_string())?;
        let all = config.players();
        if all.is_empty() {
            return Err("the scenario has no players".into());
        }
        let ais: Vec<Ai> = all
            .iter()
            .map(|&p| {
                let difficulty = players
                    .iter()
                    .find(|(q, _)| *q == p)
                    .map_or(Difficulty::Passive, |&(_, d)| d);
                Ai::new(p, difficulty, config, &map, seed)
            })
            .collect();
        let lockstep = LockstepConfig {
            players: all.clone(),
            input_delay: INPUT_DELAY,
            hash_interval: 20,
        };
        let mut session = LockstepSession::new(all[0], &lockstep);
        for &p in &all[1..] {
            session.host_player(p);
        }
        Ok(Self {
            sim,
            session,
            transport: LocalTransport,
            recorder: ReplayRecorder::new(seed, config.clone(), &all),
            commands: vec![0; ais.len()],
            ais,
            seed,
        })
    }

    pub const fn sim(&self) -> &Sim {
        &self.sim
    }

    pub fn ais(&self) -> &[Ai] {
        &self.ais
    }

    /// Lets every AI look at the current state; returns their commands in
    /// player order. Separate from [`step`](Self::step) so a caller can time
    /// the AI.
    pub fn think(&mut self) -> Vec<Vec<Command>> {
        let sim = &self.sim;
        self.ais
            .iter_mut()
            .map(|ai| ai.think(&PlayerView::new(sim, ai.player())))
            .collect()
    }

    /// Sends the commands of [`think`](Self::think) through the session and
    /// simulates one tick.
    ///
    /// # Panics
    /// Panics if the session breaks (it cannot with `LocalTransport`).
    pub fn step(&mut self, decisions: Vec<Vec<Command>>) {
        self.step_observed(decisions, |_| {});
    }

    /// [`Self::step`] with [`Sim::step_observed`]: `done` is called after
    /// every phase of the tick (benchmarks).
    ///
    /// # Panics
    /// Panics if the session breaks (it cannot with `LocalTransport`).
    pub fn step_observed(
        &mut self,
        mut decisions: Vec<Vec<Command>>,
        done: impl FnMut(sim::Phase),
    ) {
        decisions.resize(self.ais.len(), Vec::new());
        for (count, d) in self.commands.iter_mut().zip(&decisions) {
            *count += d.len();
        }
        // A loop like the game's (a raised input delay asks for several
        // frames); the first frame of every player carries the commands.
        while self.session.needs_local_input() {
            let mut frames = std::mem::take(&mut decisions).into_iter();
            self.session
                .submit_local(frames.next().unwrap_or_default(), &mut self.transport)
                .expect("local session");
            for ai in &self.ais[1..] {
                self.session
                    .submit_hosted(
                        ai.player(),
                        frames.next().unwrap_or_default(),
                        &mut self.transport,
                    )
                    .expect("local session");
            }
        }
        let commands = self.session.next_commands().expect("all frames are local");
        self.recorder.record(&commands);
        self.sim.step_observed(&commands, done);
    }

    /// Plays until the match is decided or `max_ticks` ticks were
    /// simulated.
    pub fn run(&mut self, max_ticks: u64) -> GameResult {
        while self.sim.outcome().is_none() && self.sim.tick() < max_ticks {
            let decisions = self.think();
            self.step(decisions);
        }
        self.result()
    }

    /// The result so far.
    pub fn result(&self) -> GameResult {
        let outcome = self.sim.outcome();
        GameResult {
            seed: self.seed,
            winner: outcome.and_then(|o| o.winner),
            decided_tick: outcome.map(|o| o.tick),
            ticks: self.sim.tick(),
            alive: self
                .ais
                .iter()
                .map(|ai| (ai.player(), self.sim.alive_units(ai.player())))
                .collect(),
            end_hash: self.sim.state_hash(),
            commands: self
                .ais
                .iter()
                .zip(&self.commands)
                .map(|(ai, &n)| (ai.player(), n))
                .collect(),
        }
    }

    /// The replay of the match so far.
    pub fn replay(&self) -> Replay {
        self.recorder.finish(&self.sim)
    }
}
