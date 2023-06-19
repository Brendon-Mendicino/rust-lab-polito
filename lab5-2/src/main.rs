use std::{
    ffi::OsString,
    io::{stdin, stdout, BufRead, BufReader, Read, Write},
    os::unix::prelude::OsStrExt,
    process::{Command, Stdio},
    thread, vec,
};

use crossbeam::channel::{Receiver, Sender};

#[derive(Debug)]
enum ChildState {
    Working,
    Killed,
}

struct EventLoop {
    console_rx: Receiver<String>,
    child_rx: Receiver<(ChildState, String)>,
    prog_sx: Sender<String>,
    child_sx: Sender<String>,
}

fn input_reader(console_sx: Sender<String>) {
    let mut stdin = BufReader::new(stdin());
    loop {
        let mut output = String::new();
        stdin.read_line(&mut output).unwrap();
        console_sx.send(output).unwrap();
    }
}

fn handle_child(
    prog_rx: Receiver<String>,
    child_console_rx: Receiver<String>,
    child_sx: Sender<(ChildState, String)>,
) {
    loop {
        let prog = prog_rx.recv().unwrap();

        let mut progs = prog.split_ascii_whitespace().collect::<Vec<_>>();
        println!("child: {:?}", progs);

        let mut child = Command::new(progs[0])
            .args(&mut progs[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let child_stdout = child.stdout.take().unwrap();
        let mut child_reader = BufReader::new(child_stdout);

        loop {
            let mut output = String::new();
            let bytes = child_reader.read_line(&mut output).unwrap();

            // EOF reached
            if bytes == 0 {
                child_sx.send((ChildState::Killed, output)).unwrap();
                break;
            }

            child_sx.send((ChildState::Working, output)).unwrap();

            let console = child_console_rx.try_recv().unwrap_or(String::new());
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
        }
    }
}

#[derive(Debug)]
enum LoopState {
    Prompting,
    ProgRunning,
}

#[derive(Debug)]
enum LineFrom {
    Console,
    Child,
}

fn main_event_loop(event: EventLoop) {
    let mut state = LoopState::Prompting;
    loop {
        if let LoopState::Prompting = state {
            print!("> ");
            stdout().flush().unwrap();
        }

        let prog = event.console_rx.recv().unwrap();
        event.prog_sx.send(prog).unwrap();
        state = LoopState::ProgRunning;

        while let LoopState::ProgRunning = state {
            let (from, output) = crossbeam::select! {
                recv(event.child_rx) -> line => (LineFrom::Child, line.unwrap()),
                recv(event.console_rx) -> line => (LineFrom::Console, (ChildState::Working, line.unwrap())),
            };

            if let ChildState::Killed = output.0 { state = LoopState::Prompting; }

            match from {
                LineFrom::Console => event.child_sx.send(output.1).unwrap(),
                LineFrom::Child => stdout().write_all(output.1.as_bytes()).unwrap(),
            }
            stdout().flush().unwrap();
        }
    }
}

fn main() {
    let (child_sx, child_rx) = crossbeam::channel::unbounded();
    let (father_sx, father_rx) = crossbeam::channel::unbounded();
    let (console_sx, console_rx) = crossbeam::channel::unbounded();
    let (prog_sx, prog_rx) = crossbeam::channel::unbounded();

    let event = EventLoop {
        child_rx,
        child_sx: father_sx,
        console_rx,
        prog_sx,
    };

    thread::scope(|s| {
        s.spawn(move || main_event_loop(event));
        s.spawn(move || input_reader(console_sx));
        s.spawn(move || handle_child(prog_rx, father_rx, child_sx));
    });
}
