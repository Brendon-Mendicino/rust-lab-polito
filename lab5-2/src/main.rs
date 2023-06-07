use std::{
    ffi::OsString,
    io::{stdin, stdout, BufRead, BufReader, Read, Write},
    os::unix::prelude::OsStrExt,
    process::{Command, Stdio},
    vec,
};

use crossbeam::channel::{Receiver, Sender};

struct EventLoop {
    console_rx: Receiver<String>,
    child_rx: Receiver<String>,
    prog_sx: Sender<String>,
    child_sx: Sender<String>,
}

fn input_reader(console_sx: Sender<String>) {
    loop {
        let mut output = String::new();
        stdin().read_line(&mut output);
        console_sx.send(output);
    }
}

fn handle_child(prog_rx: Receiver<String>, console_rx: Receiver<String>, child_sx: Sender<String>) {
    loop {
        let prog = prog_rx.recv().unwrap();

        let mut progs = prog.split_ascii_whitespace().collect::<Vec<_>>();

        let mut child = Command::new(progs[0])
            .args(&mut progs[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        loop {
            let console = console_rx.try_recv().unwrap_or(String::new());
            if let Some(1) = console.as_bytes().first() {
                child.kill().unwrap();
                break;
            }

            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(console.as_bytes())
                .unwrap();

            let mut output = String::new();
            match child.stdout.take() {
                Some(out) => {
                    BufReader::new(out).read_line(&mut output).unwrap();
                }
                None => (),
            }

            child_sx.send(output).unwrap();
        }
    }
}

enum LoopState {
    Prompting,
    ProgRunning,
}

enum LineFrom {
    Console,
    Prog,
}

fn main_event_loop(event: EventLoop) {
    let mut state = LoopState::Prompting;
    loop {
        if let LoopState::ProgRunning = state {
            println!("> ");
            stdout().flush().unwrap();
        }

        let (from, output) = crossbeam::select! {
            recv(event.child_rx) -> line => (LineFrom::Console, line.unwrap()),
            recv(event.console_rx) -> line => (LineFrom::Prog, line.unwrap()),
        };

        match from {
            LineFrom::Console => {
                match state {
                    LoopState::ProgRunning => event.child_sx.send(output).unwrap(),
                    LoopState::Prompting => event.prog_sx.send(output).unwrap(),
                }
                state = LoopState::ProgRunning;
            }
            LineFrom::Prog => stdout().write_all(output.as_bytes()).unwrap(),
        }
    }
}

fn main() {
    println!("Hello, world!");
}
