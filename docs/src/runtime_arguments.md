# Runtime Arguments

## Replacement Variable Notation

The replacement variable notation allows you to dynamically replace tokens in commands with contextual data–such as the user's active search query, system environment settings or Runtime Arguments.

### Available Tokens

1. `{keyword}`

   Replaces the token with the exact text currently typed into the search bar.

   #### Syntax Example:

   ```
   https://www.example.com/search?q={keyword}
   ```


2. `{terminal}`

   Automatically resolbes the user-defined or system-detected default terminal emulator.

   > [!TIP]
   > Most terminal emulators close instant once their child process finishes executing. To keep the terminal window open after your command runs,wrap the execution string like this: `{terminal} sh -c "<command>; exec $SHELL"`

   #### Syntax Example:

   ```sh
   {terminal} sh -c \"ssh {variable:user}@{variable:host}; exec $SHELL\"
   ```


3. `{variable:<name>}`

   Inserts the Exec Variable into the command. This token only works if a matching input field is explicitly declared in the configuration's `variables` array

   #### Syntax Example:

   ```json
   {
       "variables": [
           { "string_input": "query" }
       ],
       "exec": "https://example.com/search?q={variable:query}"
   }
   ```


4. `{prefix[<variable name>]:<prefix text>}`

   A conditional modifier token used used to handle optional inputs gracefully.

   - If the specified variable **contains a value**, the entire token evaluates to the `<prefix text>`.
   - If the variable **is empty or unassigned**, the entire token resolves to an emptry string `""`.

   This is highly effective for injecting optional CLI flags or toggling between a website's landing pgae and its search index.

   #### Syntax Example:

   ```json
   {
       "variables": [
           { "string_input": "query" }
       ],
       "exec": "https://example.com/{prefix[query]:search?q=}{variable:query}"
   }
   ```



<details>
<summary>**Complete Configuration Example:**</summary>

Heres a practical look at how there replacement variables mesh inside a launcher configuration file.

```json
{
    "name": "System & Network Utils",
    "type": "command",
    "args": {
        "commands": {
            "SSH Tunnel": {
                "icon": "sherlock-link",
                "variables": [
                    { "string_input": "User" },
                    { "string_input": "Host" }
                ],
                "exec": "{terminal} ssh {variable:User}@{variable:Host}",
                "search_string": "ssh"
            },
            "NordVPN Connect": {
                "icon": "nordvpn",
                "variables": [
                    { "choice": { "name": "Server", "choices": ["us", "uk", "de"] } }
                ],
                "exec": "{terminal} sh -c \"nordvpn c {variable:Server}; exec $SHELL\"",
                "search_string": "nordvpn"
            },
            "NordVPN Daemon": {
                "icon": "nordvpn",
                "exec": "systemctl --user start nordvpnd",
                "search_string": "nordvpn daemon"
            }
        }
    },
    "priority": 1
}
```

</details>

In Sherlock **Runtime Arguments** are dynamic placeholders that allow you to inject real-time arguments into your applications, scripts, and commands right at the moment you launch them. Instead of relying on hardcoded shortcuts, _Runtime Arguments_ turn your launcher into an interactive CLI shell.

## String Input

`string_input`

A plain text input field.

```json
{
                  "string_input": "repository",
                  "default": "sherlock-gpui",
                  "validation": "[a-zA-Z0-9._-]"
                }
```

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `default` | `String` |  | — | The default value this field should show |
| `validation` | `Character class` |  | — | A plain text input field. The entered value replaces `{variable:<name>}` in `exec`, where `<name>` is the value of `string_input`. |

## Number Input

`number_input`

A number input field.

```json
{ "number_input": "example"}
```

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `default` | `Number` |  | — | The default value this field should show |

## Path Input

`path_input`

A text input featuring path completion. By default, completion paths are resolved relative to the `$HOME` directory. Starting the input with a `/` prefix will search from the system root instead.

```json
{ "path_input": "path" }
```

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `default` | `String` |  | — | The default value this field should show |

## Command Input

`command_input`

A text input featuring path completion. Unlike `path_input`, this will only look at executeable files. First, it will look at the `$PATH`, then it will try to complete like `path_input`.

```json
{ "command_input": "command" }
```

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `default` | `String` |  | — | The default value this field should show |

## Choice Input

`choice`

Presents the user with a predefined list of options to select from. Each choice has a `label` shown in the UI and a `value` passed to the command.

```json
{
    "choice": {
        "name": "temperature",
        "choices": [
            {"label": "5000", "value": "5000"},
            {"label": "6000 <span color='#555555'><i>default</i></span>", "value": "6000"},
            {"label": "7000", "value": "7000"},
            {"label": "8000", "value": "8000"}
        ]
    }
}
```

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `default` | `String` |  | — | The default value this field should show |
| `default` | `Choice` |  | — | The choices that should be displayed in the dropdown menu. |

## Password Input

`password_input`

Like string_input but the value is masked in the UI.

```json
{ "password_input": "sudo" }
```

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `default` | `String` |  | — | The default value this field should show |