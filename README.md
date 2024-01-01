## Simple Ball Passing Game

This simple toy project demonstrates a basic ball passing game implemented in Rust, featuring a client-server architecture. Players can join the game, receive a unique ID, and interact with each other by passing the ball.

### Features

- [x] **Client-Server Communication:**
  - Clients establish a connection with the server over TCP.
  - Clients are assigned a unique ID upon joining the game.

- [x] **Real-time Game State Display:**
  - Clients display up-to-date information about the game state.

- [x] **Ball Passing Mechanism:**
  - Clients can pass the ball to another player.

- [x] **Server Management:**
  - Server manages multiple client connections.
  - Server accepts connections during the game.
  - Server correctly handles clients leaving the game.

### Packet Frame Design

The communication protocol for this project uses a custom design over TCP. The frame structure involves storing only the payload size in the header, minimizing packet frame size. Below is the packet frame design diagram:

```plaintext
+---------------------+
| Payload Size (u32) |
+---------------------+
|      Payload        |
+---------------------
```

This design aims to facilitate efficient communication between the server and clients, making the application layer protocol less trivial.

### Network Message API

#### Client Requests and Action Messages

1. **PassBall(Player):**
   - Request for the ball to be passed to the specified player.

2. **Leave:**
   - Notify the server that the client is leaving the game.

#### Server Events

1. **InitialState(Player, GameState):**
   - Send the current game state to a joining player.

2. **PlayerJoin(Player):**
   - Notify clients of a newly connected player.

3. **PlayerLeave(Player):**
   - Notify clients of a disconnected player.

4. **PassBall(Player, WhoPassed):**
   - Notify clients that the ball has been passed, providing the reason for the pass (WhoPassed).

5. **Disconnect(String):**
   - Notify clients that the server is shutting down, with the reason provided in the parameter.

### Getting Started

To run the project, follow these steps:

1. Clone the repository.
2. Compile the project using `cargo build`.
3. Run the server: `./target/debug/rust-ball-pass-game server`
4. Run the client: `./target/debug/rust-ball-pass-game client`

### Contribution

Feel free to contribute to the project by submitting issues or pull requests.

### License

This project is licensed under the [MIT License](LICENSE).

