use super::*;

#[test]
fn messages_and_reasons_follow_the_design_table() {
    let got: Vec<(String, String)> = all_errors()
        .iter()
        .map(|e| (e.message(), e.reason()))
        .collect();
    let expected: Vec<(&str, &str)> = vec![
        (
            "project not found.",
            "No project matches \"Alpha\". Run \"agpj project ls\" to see the projects.",
        ),
        (
            "project \"Alpha\" already exists.",
            "Project names must be unique. Choose another name.",
        ),
        (
            "task #7 not found.",
            "No task has ID 7. Run \"agpj ls\" to see the tasks.",
        ),
        (
            "task #7 has no result plan RP-9.",
            "Run \"agpj show 7 --json\" and use an ID from resultPlans.",
        ),
        (
            "result plan RP-1 of task #7 is already delivered by result v2.",
            "Pick a plan whose deliveredResultVersion is null, or record the result without --plan.",
        ),
        (
            "unexpected argument '--x'",
            "Run \"agpj add --help\" for usage.",
        ),
        ("title is invalid.", "r"),
        (
            "cannot set #11 to running.",
            "Only tasks in Doing can be running or paused. #11 is in Todo.",
        ),
        (
            "state is invalid for this assignee.",
            "Only an AI-owned task in Doing can be running or paused.",
        ),
        (
            "cannot change the state from running to idle.",
            "In Doing a task goes from idle to running, from running to paused, and from paused to running. Move the card to change the other states.",
        ),
        (
            "the project has no release version.",
            "Set one with \"version release <VERSION>\", or name a version.",
        ),
        (
            "task #5 is a draft and read-only for agents.",
            "Wait for the user to finalize it, then try again.",
        ),
        (
            "task #5 is in the Backlog and has not been sent to Todo.",
            "Send it to Todo first (agpj mv <id> todo --actor human), then try again.",
        ),
        (
            "epic E is a draft and read-only for the CLI.",
            "Wait for the user to finalize it, then try again.",
        ),
        (
            "the plan has open questions: Q-1.",
            "Check the questions, then send again with --force (the API takes ack: true).",
        ),
        (
            "start is for a person only.",
            "Ask the user to do this in the web UI or run it with --actor human.",
        ),
        (
            "task #5 is done and read-only for agents.",
            "A person must move the task out of Done before you can change it.",
        ),
        (
            "task #5 has unmet acceptance conditions: AC-2.",
            "Mark each condition done once it is verified, or ask a person to move the task.",
        ),
        (
            "task #5 is not a draft.",
            "Only a draft can be saved or finalized this way. #5 is not a draft.",
        ),
        (
            "cannot make #6 a draft while it is running.",
            "Only tasks whose agent status is idle can become drafts. #6 is not idle.",
        ),
        (
            "task #5 waits for answers to its questions.",
            "Wait for the answers with \"agpj message wait\", or see the questions with \"agpj message ls --task ID\".",
        ),
        (
            "Q-1 already answered.",
            "The first recorded answer is kept. Compare it with the answer you have, and ask again if they differ.",
        ),
        (
            "Q-2 withdrawn.",
            "A withdrawn question takes no answer. Ask a new question if one is still needed.",
        ),
        (
            "cannot send while Q-2 has no answer.",
            "Answer the questions (the board, or \"agpj message answer\"), then send again.",
        ),
        (
            "message Q-3 not found.",
            "Run \"agpj message ls --status all\" to see the messages of the project.",
        ),
        (
            "message Q-3 already has its reply.",
            "The message already has its reply; there is nothing to withdraw.",
        ),
        ("Q-3 is not a request.", "Name a request for this command."),
        (
            "request Q-4 was already accepted.",
            "Another agent accepted this request first. Leave it to that agent.",
        ),
        (
            "the answer of Q-4 was already accepted.",
            "An agent took this answer in before. Read it with \"agpj message show ID\".",
        ),
        (
            "question Q-4 has no answer yet.",
            "Only an answered question can be accepted. Wait for the answer with \"agpj message wait\".",
        ),
        (
            "request Q-4 was withdrawn.",
            "The person withdrew this request. Nothing is written to a withdrawn request.",
        ),
        (
            "withdrawing a question is for an agent only.",
            "An agent does this with --actor ai; a person has no way to do it.",
        ),
        (
            "task #5 is now done.",
            "Another change moved the task first. Check its current status and move it again.",
        ),
        (
            "the planned version of task #5 changed.",
            "Another change set the planned version first. Read the task again and choose the version again.",
        ),
        (
            "deleting a task needs confirmation from a human.",
            "Run this command yourself in a terminal. Agents cannot delete tasks or projects, or replace a board.",
        ),
        (
            "cannot read a.json.",
            "No such file or directory (os error 2)",
        ),
        (
            "projects/p/tasks/1/task.json is not valid.",
            "title: Titles must be 1 to 100 characters on one line.",
        ),
        (
            "cannot read projects/p/project.json.",
            "Permission denied (os error 13)",
        ),
        (
            "workspace.json has an unsupported format version.",
            "Only formatVersion 1 is supported, found 2. The file is not converted.",
        ),
        (
            "the choice is not part of this question.",
            "Choose one of the choices of the question.",
        ),
        (
            "the board is busy.",
            "Another agpj command is writing. Try again.",
        ),
        ("cannot open the board.", "boom"),
        (
            "the board is already being served.",
            "Open http://127.0.0.1:7420/ or stop the other \"agpj serve\".",
        ),
        (
            "port 7420 is already in use.",
            "Choose another port with --port.",
        ),
        (
            "local network connections are not allowed.",
            "This environment does not allow local network connections, so the Agent Project server cannot be started or reached. In Codex, run \"agpj setup\" to allow agpj to run outside the sandbox.",
        ),
        ("the board is unavailable.", "no running server"),
        ("the cursor is invalid.", "wrong project"),
        (
            "no board change before the timeout.",
            "Nothing changed in the project. This is not an answer; read the snapshot before acting.",
        ),
        (
            "the document path is invalid.",
            "Paths are relative and use /.",
        ),
        (
            "document a/b.md not found.",
            "No such file in the project's Docs. Choose a file from the tree.",
        ),
        (
            "a.txt is not a Markdown file.",
            "Only files ending in .md or .mdx can be opened.",
        ),
        (
            "a.md is too large.",
            "Files larger than 2097152 bytes cannot be opened here.",
        ),
        (
            "a.md is not UTF-8 text.",
            "Only UTF-8 text can be opened here. The file is not changed.",
        ),
        (
            "a.svg is not a supported image.",
            "Only PNG, JPEG, WebP, GIF, AVIF and SVG images are served.",
        ),
        (
            "this document operation is not available yet.",
            "The Docs API is still being built. Try again in a later version.",
        ),
        ("cannot open the Docs folder.", "xdg-open is not installed"),
        (
            "did not write docs/release/v1.8.0.md.",
            "The file changed while it was generated.",
        ),
        (
            "the ngrok connection is not available.",
            "No server is running.",
        ),
        (
            "the ngrok domain is not valid.",
            "Give a host name only, such as example.ngrok-free.dev, without https://, a path, or a port.",
        ),
        (
            "secure storage on this computer is not available.",
            "The keychain is not available.",
        ),
        (
            "cannot open the browser.",
            "Cannot start xdg-open: No such file or directory (os error 2). Open http://127.0.0.1:7420/?project=0123456789abcdef0123456789abcdef in your browser yourself.",
        ),
        (
            "the claude command is not installed.",
            "Install claude and make it available on the PATH of the server, then try again.",
        ),
        (
            "codex is not logged in.",
            "Log in to codex in a terminal, then try again.",
        ),
        (
            "the session was not launched.",
            "A board-launched claude-code session already exists.",
        ),
        (
            "the session could not be started.",
            "claude could not be started.",
        ),
        (
            "the project has no bound folder.",
            "Bind a folder to project demo with \"agpj project bind\" in that folder.",
        ),
    ];
    let expected: Vec<(String, String)> = expected
        .into_iter()
        .map(|(m, r)| (m.to_string(), r.to_string()))
        .collect();
    assert_eq!(got, expected);
}

#[test]
fn port_in_use_names_what_holds_the_port() {
    let board = AgpjError::PortInUse {
        port: 7420,
        holder: crate::error::PortHolder::AgentBoard { pid: 41235 },
    };
    assert_eq!(
        board.reason(),
        "Port 7420 is used by another Agent Project server (pid 41235) with a different data folder. Choose another port with --port."
    );
    let other = AgpjError::PortInUse {
        port: 7420,
        holder: crate::error::PortHolder::OtherProgram,
    };
    assert_eq!(
        other.reason(),
        "Port 7420 is used by another program. Choose another port with --port."
    );
}

#[test]
fn already_serving_says_when_the_server_was_started_automatically() {
    let auto = AgpjError::AlreadyServing {
        port: 52114,
        auto: true,
    };
    assert!(
        auto.reason().contains("started automatically"),
        "{}",
        auto.reason()
    );
    assert!(auto.reason().contains("agpj serve -d"), "{}", auto.reason());
}
