# TermiGit 🦀

A blazingly fast Git client for your terminal, built with Rust. Interactively view logs, manage the staging area, see detailed diffs, and perform commit and push operations with SSH key support. Manage your repositories without ever leaving the command line.


## Objective

The goal of this project is to create a fast, efficient, and keyboard-driven Git client that runs directly in the terminal. It aims to provide the most common Git functionalities (logging, diffing, staging, committing, pushing) in a clean and responsive Terminal User Interface (TUI), demonstrating a practical application of Rust for building complex, real-world command-line tools.


## Project Structure

The codebase is organized into modules with a clear separation of concerns:

```
.
└── src
   ├── main.rs          \# Entrypoint, terminal setup, and main async event loop.
   ├── app.rs           \# Defines application state (App struct) and handles input logic.
   ├── ui.rs            \# All rendering logic to draw the TUI.
   ├── git.rs           \# Encapsulates all backend interactions with the git2 library.
   └── types/           \# Contains simple data structures for commits and statuses.
       ├── mod.rs
       ├── commit\_info.rs
       └── status\_info.rs
```
* **`main.rs`**: Initializes the terminal, creates the `App` state object, and runs the main event loop.
* **`app.rs`**: The "brain" of the application. It holds all state, including UI selection, active panels, and input modes. It processes key events and calls the appropriate backend functions.
* **`ui.rs`**: The "view" layer. It is responsible for drawing all widgets to the screen based on the current state of the `App` struct.
* **`git.rs`**: The "model" or backend layer. It contains all functions that interact directly with a Git repository using the `git2` crate.
* **`types/`**: A directory for simple, plain data structs that decouple the application logic from the `git2` library's complex types.


## Working of the Code

The application operates on an asynchronous event loop managed by `tokio` in **`main.rs`**.

1.  **Initialization**: The `main` function sets up the terminal in "raw mode," creates an instance of `App` from **`app.rs`**, and starts the main loop. The `App` struct initializes its state by fetching the initial commit log and file statuses using functions from **`git.rs`**.

2.  **Event Loop**: The main loop uses `tokio::select!` to simultaneously listen for two types of events without blocking:
    * **User Input**: Keyboard presses from the terminal.
    * **Async Messages**: Feedback from long-running background tasks (like a Git push).

3.  **State Management**: When a key press is detected, it's passed to `app.handle_key_event()`. This method updates the `App` struct's state (e.g., changes the selected item, switches the active panel, or enters "Commit Input" mode). For Git operations, it calls the relevant function in **`git.rs`**.

4.  **Rendering**: On every iteration of the loop, the `terminal.draw()` method is called. It passes the current `App` state to the `draw()` function in **`ui.rs`**. The UI module then renders all the panels, lists, and popups based on the data and state it received.

5.  **Async Operations**: For potentially long-running, blocking tasks like `git push`, we use `tokio::task::spawn_blocking`. This moves the non-thread-safe `git2` operation to a dedicated thread, preventing the UI from freezing. Once the task is complete, it sends a message back to the main event loop via an MPSC channel to update the UI with the result (e.g., "Push successful!").
