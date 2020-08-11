# StatusBot
> Slack bot to track user and team location 

StatusBot provides a way for managers or team leads to quickly find the status of a user.  It monitors a slack channel (say, `#daily_status`) for updates which can then be queried through the `/location` command.

![](header.png)

## Installation

Windows & OS X & Linux:

```sh
git clone ...
```
## Commands

| Command                                   | Description                                                 |
| ----------------------------------------- | ----------------------------------------------------------- |
| `/location <username>`                      | Prints the status for a user                                |
| `/location <team_name>`                     | Prints the status of all members beloning to a team         |
| `/location team create <team_name>`         | Creates a new team with name `team_name`                      |
| `/location team delete <team_name>     `    | Deletes a team with name `team_name`.  **This cannot be undone**  |
| `/location team <team_name> add <username>` | Adds a user to a team                                       |
| `/location team <team_name> del <username>` | Removes a user from a team                                  |

## Usage example

Query status of user "Obi-Wan":
```sh
/location @Obi-Wan
```

Query status of team "Senate":
```shA
/location Senate
```

Create a new team with the name "IAmTheSenate"
```sh
/location team create IAmTheSenate
```

Add a new member to the team:
```sh
/location team IAmTheSenate add Palpatine
```
## Development setup

Standard Rust development procedure.

```sh
cargo build
cargo run
```

## Release History

* 0.0.1
    * Work in progress

## Meta

Kevin Allison

Distributed under the MIT license. See ``LICENSE`` for more information.

[https://github.com/kvnallsn/statusbot]

## Contributing

1. Fork it (<https://github.com/kvnallsn/statusbot/fork>)
2. Create your feature branch (`git checkout -b feature/fooBar`)
3. Commit your changes (`git commit -am 'Add some fooBar'`)
4. Push to the branch (`git push origin feature/fooBar`)
5. Create a new Pull Request

<!-- Markdown link & img dfn's -->

