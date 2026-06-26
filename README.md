# ebc-hub

Remote control and battery test management hub for ZKETECH EBC battery testers.

`ebc-hub` is a Rust-based application for managing one or more ZKETECH EBC battery testers from a Raspberry Pi, server, or desktop machine.

Unlike a traditional desktop application, `ebc-hub` is designed around a long-running server process. Battery tests continue running independently of any connected browser or CLI client.

## Goals

- Control multiple EBC devices simultaneously
- Long-running autonomous battery tests
- Persistent SQLite storage
- Battery inventory management
- Complete test history
- Remote control via WebSocket API
- Browser-based user interface
- CSV export and data analysis

## Current Status

The project is in active development.

### Implemented

- EBC protocol implementation
- Multi-device manager
- Device connect/disconnect
- Start / Stop / Adjust / Continue commands
- Event broadcasting
- SQLite database
- SQL migrations
- Type-safe database access using `sqlx`
- Battery type management
- Battery inventory
- Battery intake management
- Interactive CLI

### In Progress

- Test management
- Test sessions
- Sample storage
- Persistent test runner
- HTTP/WebSocket server

### Planned

- Browser UI
- Live monitoring
- CSV import/export
- Automatic report generation
- REST API
- Authentication

## Database

The application already contains a normalized database schema for:

- Battery types
- Batteries
- Battery intake information
- Tests
- Test sessions
- Samples

This allows battery metadata, delivery measurements and complete test histories to be stored independently.

## Supported Devices

Currently tested with:

- ZKETECH EBC-A20

Support for additional ZKETECH EBC models is planned.

## Example CLI

```text
battery-type add EVE LF314 LiFePO4 3200 314000

battery add eve-314ah-001 1

battery-intake set eve-314ah-001 3291 180

connect 1

start 1 DSC-CC 20000 2500 0
```

## Roadmap

- [x] EBC communication
- [x] Multi-device manager
- [x] SQLite storage
- [x] Battery management
- [x] Battery intake management
- [ ] Test runner
- [ ] HTTP/WebSocket server
- [ ] Browser UI
- [ ] CSV export
- [ ] Docker deployment

## License

This project is licensed under the MIT License.

## Acknowledgements

This project was inspired by and partially based on the protocol work from
[Kazhuu/ebc-battery-tester](https://github.com/Kazhuu/ebc-battery-tester),
which reverse-engineered and documented the ZKETECH EBC-A20 serial protocol.

The original project is licensed under the MIT License. Protocol details and parts of the device communication implementation were used as a reference while building this project.