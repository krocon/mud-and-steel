//! Acceptance test of prompt 025: two separate `cli` processes play the demo
//! over real UDP on localhost. The client finds the host by the LAN search
//! (broadcast query, answered for localhost too).

use std::net::UdpSocket;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const TICKS: u64 = 600;
const SEED: u64 = 42;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cli"))
}

/// A UDP port that was free a moment ago.
fn free_port() -> u16 {
    UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn spawn(command: &mut Command) -> Child {
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

/// Waits for the process, killing it after a minute.
fn finish(mut child: Child) -> Output {
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(60) {
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    child.wait_with_output().unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Value of `key=` in the result line.
fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.split_whitespace()
        .find_map(|word| word.strip_prefix(key)?.strip_prefix('='))
}

fn scratch_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mud-and-steel-lan-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Starts host and client; the client searches the LAN unless `search` is
/// false, then it joins by address.
fn play(extra_host: &[&str], extra_join: &[&str], search: bool) -> (Output, Output) {
    let port = free_port().to_string();
    let discovery = free_port().to_string();
    let ticks = TICKS.to_string();
    let host = spawn(
        cli()
            .args(["lan-host", "--port", &port, "--discovery-port", &discovery])
            .args(["--ticks", &ticks, "--timeout-s", "50"])
            .args(extra_host),
    );
    let mut join = cli();
    join.args(["lan-join", "--ticks", &ticks, "--timeout-s", "50"]);
    if search {
        join.args(["--discovery-port", &discovery]);
    } else {
        join.args(["--address", &format!("127.0.0.1:{port}")]);
    }
    let join = spawn(join.args(extra_join));
    (finish(host), finish(join))
}

#[test]
fn two_processes_play_without_desync() {
    let (host, join) = play(&[], &[], true);
    let (host_text, join_text) = (text(&host), text(&join));
    assert!(host.status.success(), "host failed:\n{host_text}");
    assert!(join.status.success(), "client failed:\n{join_text}");
    assert!(join_text.contains("found 'cli host' at"), "{join_text}");

    let expected = format!(
        "{:#018x}",
        sim::scenario::run_demo(SEED, TICKS).state_hash()
    );
    for text in [&host_text, &join_text] {
        assert_eq!(field(text, "ticks"), Some("600"), "{text}");
        assert_eq!(field(text, "verified"), Some("600"), "{text}");
        assert_eq!(field(text, "hash"), Some(expected.as_str()), "{text}");
    }
    assert_eq!(field(&host_text, "player"), Some("0"));
    assert_eq!(field(&join_text, "player"), Some("1"));
}

#[test]
fn a_desync_is_reported_by_both_with_replays() {
    let dir = scratch_dir();
    let host_replay = dir.join("host.msreplay");
    let join_replay = dir.join("join.msreplay");
    let (host, join) = play(
        &["--record", host_replay.to_str().unwrap()],
        &[
            "--record",
            join_replay.to_str().unwrap(),
            "--cheat-tick",
            "55",
        ],
        false,
    );
    let (host_text, join_text) = (text(&host), text(&join));
    assert_eq!(host.status.code(), Some(1), "{host_text}");
    assert_eq!(join.status.code(), Some(1), "{join_text}");
    // Hashes are exchanged every 20 ticks: the first mismatch is at tick 60.
    assert!(host_text.contains("DESYNC player=0 tick=60"), "{host_text}");
    assert!(join_text.contains("DESYNC player=1 tick=60"), "{join_text}");

    // Both replays play back to their own end hash; the client's contains
    // the extra command, which is how the replays show the cause.
    for path in [&host_replay, &join_replay] {
        let output = cli().arg("replay").arg(path).output().unwrap();
        assert!(output.status.success(), "{}", text(&output));
    }
    let _ = std::fs::remove_dir_all(dir);
}
