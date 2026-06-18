# Launchers

Sherlock separates **Launchers** (The Logic) from **Widgets** (The View). One Launcher configuration can generate multiple Widgets (like a weather tile and a clock tile), but they all follow the same `priority` and `home` rules.

```
[Weather Launcher]
    [Widget] Weather Display
[App Launcher]
    [Widget] App 1
    [Widget] App 2
    [Widget] App 3
```

The Widgets get sorted based by a tiered sort: `Launcher Priority` then `Search Score` then `Number of Executions`

## Shared Launcher Configuration

### Fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `name` | `string` |  | — | Display name of the launcher. Shown in the widget if set. |
| `alias` | `string` |  | — | Short trigger prefix (e.g. `app`) that scopes the launcher to alias-only mode. |
| `type` | `LauncherVariant` | ✓ | — | The category and functional variant of the launcher. |
| `priority` | `u16` | ✓ | — | Display order weight. Lower values appear first. `0` only appears in alias mode. |
| `limit` | `u16` |  | — | The number of items to display per launcher. Useful to limit the number of search results to the best `n` items. |
| `home` | `HomeType` |  | Home | Controls when the launcher is shown: `Home`, `OnlyHome`, `Search`, or `Persist`. |
| `exit` | `bool` |  | true | Whether Sherlock closes after the launcher is executed. |
| `shortcut` | `bool` |  | true | Whether a UI shortcut key is assigned to this launcher. |
| `spawn_focus` | `bool` |  | true | Whether this launcher can receive spawned focus. |
| `on_return` | `string` |  | — | Command or action executed when the user confirms this launcher. |
| `args` | `object` |  | {} | Launcher-specific arguments. Shape depends on the launcher type. |
| `binds` | `Bind[]` |  | — | Key bindings attached to this launcher. |
| `actions` | `ApplicationAction[]` |  | — | Primary context menu actions. Overwrites any actions defined in desktop files. |
| `add_actions` | `ApplicationAction[]` |  | — | Supplementary actions appended to the primary action list. |
| `variables` | `ExecVariable[]` |  | — | Runtime variable substitutions available to this launcher's commands. |

---

## App Launcher

`type = apps`

Launches installed desktop applications

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `use_keywords` | `bool` |  | true | Whether the search should use the keywords defined in the .desktop file. |

### Examples

_Basic app launcher_

```json
{
    "name": "App Launcher",
    "alias": "app",
    "type": "apps",
    "args": {
        "use_keywords": false
    },
    "priority": 4,
    "home": "Home"
}
```

---

## Bookmark Launcher

`type = bookmarks`

Launches browser bookmarks in your default browser.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `browser` | `string` |  | Default browser | The browser from which the bookmarks should be parsed |

### Examples

_Basic bookmarks launcher_

```json
{
    "name": "Bookmarks",
    "type": "bookmarks",
    "alias": "bm",
    "args": {
        "browser": "brave"
    },
    "priority": 7
}
```

---

## Calculator Launcher

`type = calculator`

Allows math calculations and different unit conversions.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `capabilities` | `Capability[]` |  | [ "calc.units", "calc.math" ] | The capabilities the calculator should have. |
| `currency_update_interval` | `u64` |  | 1440 | Number of minutes to keep the currency cache alive. |

<details>
<summary><strong>Capabilities:</strong></summary>

Capabilities control what the calculator can compute. Pass them via the `capabilities` arg:

```json
{ "capabilities": ["calc.math", "calc.units"] }
```

<details>
<summary>Math</summary>

`calc.math`

</details>

<details>
<summary>Colors</summary>

`colors`

</details>

<details>
<summary>Currency</summary>

`calc.currencies`

| Unit | Aliases | Symbol |
|---|---|---|
| Usd | usd, dollar, dollars, bucks, $ | $ |
| Eur | eur, euro, euros, € | € |
| Jpy | jpy, yen, japanese yen, ¥ | ¥ |
| Gbp | gbp, pound, pounds, sterling, £ | £ |
| Aud | aud, australian dollar, aussie, a$ | A$ |
| Cad | cad, canadian dollar, loonie, c$ | C$ |
| Chf | chf, swiss franc, franc | CHF |
| Cny | cny, chinese yuan, renminbi, yuan | ¥ |
| Nzd | nzd, new zealand dollar, kiwi, nz$ | NZ$ |
| Sek | sek, swedish krona, krona, kr | kr |
| Nok | nok, norwegian krone, krone | kr |
| Mxn | mxn, mexican peso, peso, mex$ | Mex$ |
| Sgd | sgd, singapore dollar, s$ | S$ |
| Hkd | hkd, hong kong dollar, hk$ | HK$ |
| Krw | krw, south korean won, won, ₩ | ₩ |
| Pln | pln, polish, złoty, zł | zł |
| Pen | pen, peruvian, sole, soles | S/ |

</details>

<details>
<summary>Length</summary>

`calc.length`

| Unit | Aliases | Symbol |
|---|---|---|
| Millimeter | mm, millimeter, millimeters | mm |
| Centimeter | cm, centimeter, centimeters | cm |
| Meter | m, meter, meters | m |
| Kilometer | km, kilometer, kilometers, kilos | km |
| Inch | in, inch, inches, " | in |
| Feet | ft, feet, foot, ' | ft |
| Yard | yd, yard, yards | yd |
| Mile | mi, mile, miles | mi |
| NauticalMile | nm, nautical mile | nmi |

</details>

<details>
<summary>Volume</summary>

`calc.volume`

| Unit | Aliases | Symbol |
|---|---|---|
| Milliliter | ml, milliliter, milliliters, cc | ml |
| Centiliter | cl, centiliter | cl |
| Liter | l, liter, liters | l |
| Kiloliter | kl, kiloliter | kl |
| CubicMeter | m3, cubic meter, cubic meters | m³ |
| Teaspoon | tsp, teaspoon | tsp |
| Tablespoon | tbsp, tablespoon | tbsp |
| FluidOunce | fl oz, fluid ounce, fluid ounces | fl oz |
| Cup | cup, cups | cup |
| Pint | pt, pint, pints | pt |
| Quart | qt, quart, quarts | qt |
| Gallon | gal, gallon, gallons | gal |
| ImperialGallon | imp gal | imp gal |

</details>

<details>
<summary>Weight</summary>

`calc.weight`

| Unit | Aliases | Symbol |
|---|---|---|
| Milligram | mg, milligram, milligrams | mg |
| Gram | g, gram, grams | g |
| Kilogram | kg, kilogram, kilograms, kilo, kilos | kg |
| MetricTon | t, tonne, metric ton, metric tons | t |
| Ounce | oz, ounce, ounces | oz |
| Pound | lb, lbs, pound, pounds | lb |
| Stone | st, stone, stones | st |
| ShortTon | ton, tons, us ton | ton |
| LongTon | imperial ton, uk ton | ton |
| TroyOunce | ozt, troy ounce, troy ounces | ozt |

</details>

<details>
<summary>Temperature</summary>

`calc.temperature`

| Unit | Aliases | Symbol |
|---|---|---|
| Celsius | c, celsius, °c, ° | °C |
| Fahrenheit | f, fahrenheit, °f | °F |

</details>

<details>
<summary>Pressure</summary>

`calc.pressure`

| Unit | Aliases | Symbol |
|---|---|---|
| Pascal | pa, pascal, pascals | Pa |
| Kilopascal | kpa, kilopascal | kPa |
| Bar | bar, bars | bar |
| Atmosphere | atm, atmosphere, atmospheres | atm |
| Psi | psi, pounds per square inch | psi |
| Torr | torr, mmhg | mmHg |

</details>

<details>
<summary>Digital</summary>

`calc.digital`

| Unit | Aliases | Symbol |
|---|---|---|
| Bit | bit, bits, b | bit |
| Kilobit | kb, kilobit | kb |
| Megabit | mb, megabit | Mb |
| Gigabit | gb, gigabit | Gb |
| Byte | byte, bytes, B | B |
| Kilobyte | kb, kilobyte, KB | KB |
| Megabyte | mb, megabyte, MB | MB |
| Gigabyte | gb, gigabyte, GB | GB |
| Terabyte | tb, terabyte, TB | TB |
| Petabyte | pb, petabyte, PB | PB |

</details>

<details>
<summary>Time</summary>

`calc.time`

| Unit | Aliases | Symbol |
|---|---|---|
| Milliseconds | ms, millisecond, milliseconds | ms |
| Seconds | s, sec, second, seconds | s |
| Minutes | m, min, minute, minutes | min |
| Hours | h, hr, hour, hours | h |
| Days | d, day, days | d |
| Weeks | wk, week, weeks | wk |
| Months | mo, month, months | mo |
| Years | yr, year, years | yr |

</details>

<details>
<summary>Area</summary>

`calc.area`

| Unit | Aliases | Symbol |
|---|---|---|
| SquareMeter | m2, sq m, sq meter | m² |
| SquareKilometer | km2, sq km | km² |
| SquareFoot | ft2, sq ft, sq feet | ft² |
| SquareInch | in2, sq in | in² |
| Acre | acre, acres | ac |
| Hectare | ha, hectare | ha |

</details>

<details>
<summary>Speed</summary>

`calc.speed`

| Unit | Aliases | Symbol |
|---|---|---|
| MetersPerSecond | ms, m/s, meters per second | m/s |
| KilometersPerHour | kmh, km/h, kph | km/h |
| MilesPerHour | mph, mile per hour, miles per hour | mph |
| Knot | kn, knot, knots | kn |

</details>

</details>

### Examples

_Basic calculator config_

```json
{
    "name": "Calculator",
    "type": "calculator",
    "alias": "calc",
    "args": {
        "currency_update_interval": 60,
        "capabilities": [
            "calc.math",
            "calc.units",
            "calc.currencies",
            "colors"
        ]
    },
    "priority": 1,
    "on_return": "copy"
}
```

---

## Category Launcher

`type = categories`

Applies aliases to restrict search to certain launchers.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `categories` | `{Name: AppData}` | ✓ | — | The available categories. On execution, will apply the aslias, privodes as the `exec` field. |

### Examples

_Power Menu Example_

```json
{
    "name": "Categories",
    "alias": "cat",
    "type": "categories",
    "args": {
        "categories": {
            "Power Menu": {
                "icon": "battery-full-symbolic",
                "icon_class": "reactive",
                "exec": "pm",
                "search_string": "powermenu;",
                "actions": [
                    {
                        "name": "Shutdown",
                        "icon": "system-shutdown",
                        "exec": "systemctl poweroff",
                        "method": "command"
                    },
                    {
                        "name": "Sleep",
                        "icon": "system-suspend",
                        "exec": "systemctl suspend",
                        "method": "command"
                    },
                    {
                        "name": "Lock",
                        "icon": "system-lock-screen",
                        "exec": "systemctl suspend & swaylock",
                        "method": "command"
                    },
                    {
                        "name": "Reboot",
                        "icon": "system-reboot",
                        "exec": "systemctl reboot",
                        "method": "command"
                    },
                    {
                        "name": "Log Out",
                        "icon": "system-log-out",
                        "exec": "hyprctl dispatch exit",
                        "method": "command"
                    }
                ]
            }
        }
    },
    "priority": 4,
    "home": "Home"
}
```

---

## Clipboard Launcher

`type = clipboard`

Executes commands based on the clipboard content.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `capabilities` | `Capabilities[]` |  | [ "calc.units", "calc.math" ] | The capabilities the clipboard executor should have. |

<details>
<summary><strong>Capabilities:</strong></summary>

Capabilities control what the calculator can compute. Pass them via the `capabilities` arg:

```json
{ "capabilities": ["calc.math", "calc.units"] }
```

<details>
<summary>Math</summary>

`calc.math`

</details>

<details>
<summary>Colors</summary>

`colors`

</details>

<details>
<summary>Currency</summary>

`calc.currencies`

| Unit | Aliases | Symbol |
|---|---|---|
| Usd | usd, dollar, dollars, bucks, $ | $ |
| Eur | eur, euro, euros, € | € |
| Jpy | jpy, yen, japanese yen, ¥ | ¥ |
| Gbp | gbp, pound, pounds, sterling, £ | £ |
| Aud | aud, australian dollar, aussie, a$ | A$ |
| Cad | cad, canadian dollar, loonie, c$ | C$ |
| Chf | chf, swiss franc, franc | CHF |
| Cny | cny, chinese yuan, renminbi, yuan | ¥ |
| Nzd | nzd, new zealand dollar, kiwi, nz$ | NZ$ |
| Sek | sek, swedish krona, krona, kr | kr |
| Nok | nok, norwegian krone, krone | kr |
| Mxn | mxn, mexican peso, peso, mex$ | Mex$ |
| Sgd | sgd, singapore dollar, s$ | S$ |
| Hkd | hkd, hong kong dollar, hk$ | HK$ |
| Krw | krw, south korean won, won, ₩ | ₩ |
| Pln | pln, polish, złoty, zł | zł |
| Pen | pen, peruvian, sole, soles | S/ |

</details>

<details>
<summary>Length</summary>

`calc.length`

| Unit | Aliases | Symbol |
|---|---|---|
| Millimeter | mm, millimeter, millimeters | mm |
| Centimeter | cm, centimeter, centimeters | cm |
| Meter | m, meter, meters | m |
| Kilometer | km, kilometer, kilometers, kilos | km |
| Inch | in, inch, inches, " | in |
| Feet | ft, feet, foot, ' | ft |
| Yard | yd, yard, yards | yd |
| Mile | mi, mile, miles | mi |
| NauticalMile | nm, nautical mile | nmi |

</details>

<details>
<summary>Volume</summary>

`calc.volume`

| Unit | Aliases | Symbol |
|---|---|---|
| Milliliter | ml, milliliter, milliliters, cc | ml |
| Centiliter | cl, centiliter | cl |
| Liter | l, liter, liters | l |
| Kiloliter | kl, kiloliter | kl |
| CubicMeter | m3, cubic meter, cubic meters | m³ |
| Teaspoon | tsp, teaspoon | tsp |
| Tablespoon | tbsp, tablespoon | tbsp |
| FluidOunce | fl oz, fluid ounce, fluid ounces | fl oz |
| Cup | cup, cups | cup |
| Pint | pt, pint, pints | pt |
| Quart | qt, quart, quarts | qt |
| Gallon | gal, gallon, gallons | gal |
| ImperialGallon | imp gal | imp gal |

</details>

<details>
<summary>Weight</summary>

`calc.weight`

| Unit | Aliases | Symbol |
|---|---|---|
| Milligram | mg, milligram, milligrams | mg |
| Gram | g, gram, grams | g |
| Kilogram | kg, kilogram, kilograms, kilo, kilos | kg |
| MetricTon | t, tonne, metric ton, metric tons | t |
| Ounce | oz, ounce, ounces | oz |
| Pound | lb, lbs, pound, pounds | lb |
| Stone | st, stone, stones | st |
| ShortTon | ton, tons, us ton | ton |
| LongTon | imperial ton, uk ton | ton |
| TroyOunce | ozt, troy ounce, troy ounces | ozt |

</details>

<details>
<summary>Temperature</summary>

`calc.temperature`

| Unit | Aliases | Symbol |
|---|---|---|
| Celsius | c, celsius, °c, ° | °C |
| Fahrenheit | f, fahrenheit, °f | °F |

</details>

<details>
<summary>Pressure</summary>

`calc.pressure`

| Unit | Aliases | Symbol |
|---|---|---|
| Pascal | pa, pascal, pascals | Pa |
| Kilopascal | kpa, kilopascal | kPa |
| Bar | bar, bars | bar |
| Atmosphere | atm, atmosphere, atmospheres | atm |
| Psi | psi, pounds per square inch | psi |
| Torr | torr, mmhg | mmHg |

</details>

<details>
<summary>Digital</summary>

`calc.digital`

| Unit | Aliases | Symbol |
|---|---|---|
| Bit | bit, bits, b | bit |
| Kilobit | kb, kilobit | kb |
| Megabit | mb, megabit | Mb |
| Gigabit | gb, gigabit | Gb |
| Byte | byte, bytes, B | B |
| Kilobyte | kb, kilobyte, KB | KB |
| Megabyte | mb, megabyte, MB | MB |
| Gigabyte | gb, gigabyte, GB | GB |
| Terabyte | tb, terabyte, TB | TB |
| Petabyte | pb, petabyte, PB | PB |

</details>

<details>
<summary>Time</summary>

`calc.time`

| Unit | Aliases | Symbol |
|---|---|---|
| Milliseconds | ms, millisecond, milliseconds | ms |
| Seconds | s, sec, second, seconds | s |
| Minutes | m, min, minute, minutes | min |
| Hours | h, hr, hour, hours | h |
| Days | d, day, days | d |
| Weeks | wk, week, weeks | wk |
| Months | mo, month, months | mo |
| Years | yr, year, years | yr |

</details>

<details>
<summary>Area</summary>

`calc.area`

| Unit | Aliases | Symbol |
|---|---|---|
| SquareMeter | m2, sq m, sq meter | m² |
| SquareKilometer | km2, sq km | km² |
| SquareFoot | ft2, sq ft, sq feet | ft² |
| SquareInch | in2, sq in | in² |
| Acre | acre, acres | ac |
| Hectare | ha, hectare | ha |

</details>

<details>
<summary>Speed</summary>

`calc.speed`

| Unit | Aliases | Symbol |
|---|---|---|
| MetersPerSecond | ms, m/s, meters per second | m/s |
| KilometersPerHour | kmh, km/h, kph | km/h |
| MilesPerHour | mph, mile per hour, miles per hour | mph |
| Knot | kn, knot, knots | kn |

</details>

</details>

### Examples

_Power Menu Example_

```json
{
    "name": "Clipboard",
    "type": "clipboard",
    "args": {
        "capabilities": [
            "url",
            "colors",
            "calc.math"
        ]
    },
    "on_return": "copy",
    "priority": 3,
    "home": "OnlyHome"
}
```

---

## Command Launcher

`type = commands`

Launches user-specified commands.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `commands` | `{Name: AppData}` | ✓ | — | The commands to show in Sherlock. |

### Examples

_Basic command launcher_

```json
{
    "name": "Throw Confetti",
    "type": "commands",
    "args": {
        "commands": {
            "Confetti": {
                "icon": "sherlock-confetti",
                "exec": "confetti",
                "search_string": "confetti;party"
            }
        }
    },
    "priority": 4
}
```

---

## Debug Launcher

`type = debug`

Execute different debug functions like clearing the cache or app counts.

### Inner Functions

| Name | Identifier | Description |
|---|---|---|
| Clear Cache | `inner.clear_cache` | Clears the entire ~/.cache/sherlock/ directory. |
| Clear App Counts | `inner.clear_app_counts` | Clears the app count file to reset the sorting based on execution counts. |
| Clear Errors | `inner.clear_errors` | Clears the messages from the message view |
| Insert Test Errors | `inner.insert_test_errors` | Inserts test messages for each message type: Info, Warning, and Error. |

### Examples

_Basic app launcher_

```json
{
    "name": "Debug",
    "type": "debug",
    "alias": "debug",
    "args": {},
    "priority": 1,
    "exit": false
}
```

---

## Dmenu Launcher

`type = dmenu`

The launcher to handle Dmenu-style piping

---

## Emoji Picker

`type = emoji`

A emoji picker allowing for skin tone selection.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `default_skin_tone` | `SkinTone` |  | Simpsons | The skin tone to use as the default. Can be either: Light, MediumLight, Medium, MediumDark, Dark, or Simpsons |

### Examples

_Basic emoji picker_

```json
{
    "name": "Emoji Picker",
    "alias": "emj",
    "type": "emoji",
    "args": {
        "default_skin_tone": "Simpsons"
    },
    "priority": 5,
    "home": "Home"
}
```

---

## Event Launcher

`type = event`

Shows upcoming events and joins them on return.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `look_back` | `Time` |  | 10mins | The duration events should stay visible after already having started. |
| `look_ahead` | `Time` |  | 1h | The duration events should show before having started. |

### Inner Functions

| Name | Identifier | Description |
|---|---|---|
| Hard Refresh | `inner.hard_refresh` | Refetch the events from the server. |
| Join Meeting | `inner.join_meeting` | Only acailable if its actually a meeting. Will join a meeting using the application provided by a mime-type lookup. |

### Examples

_Basic event launcher_

```json
{
    "type": "event",
    "args": {
        "look_ahead": "5 hours",
        "look_back": "50 hours"
    },
    "priority": 3,
    "home": "OnlyHome",
    "spawn_focus": false,
    "shortcut": false
}
```

---

## File Launcher

`type = files`

A file search. Allows you to search for files and directories from within Sherlock.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `backend` | `string` |  | fd | The backend to be used by the file search. Can be either of: `Fd`, `Rg`, or `WalkDir` |
| `poll_interval` | `u64` |  | 50 | The time in milliseconds between backend calls. |
| `max_results` | `usize` |  | 50 | The maximum number of results to show in the file search. |
| `path` | `path` |  | ~/ | The root path from which to start the file search. |

### Examples

_Basic event launcher_

```json
{
    "name": "File Search",
    "type": "files",
    "alias": "fs",
    "args": {
        "max_results": 50,
        "poll_interval": 50,
        "backend": "fd",
        "path": "~/"
    },
    "priority": 5,
    "home": "Home"
}
```

---

## Message Launcher

`type = message`

The launcher to provide the message view

---

## Music Player Launcher

`type = music_player`

Shows the currently played song or video with thumbnail, title, and artists.

### Inner Functions

| Name | Identifier | Description |
|---|---|---|
| Toggle Playback | `inner.toggle_playback` | Toggles current media playback status (playing/paused). |
| Previous | `inner.previous` | Skips to the previous audio element (song, video). |
| Next | `inner.next` | Skips to the next audio element (song, video). |

### Examples

_Basic music player_

```json
{
    "name": "Spotify",
    "type": "music_player",
    "args": {},
    "priority": 2,
    "home": "OnlyHome",
    "spawn_focus": false,
    "exit": false,
    "binds": [
        {
            "bind": "ctrl-l",
            "callback": "next",
            "exit": false
        },
        {
            "bind": "ctrl-h",
            "callback": "previous",
            "exit": false
        }
    ]
}
```

---

## Plugin Launcher

`type = plugin`

Launches installed desktop applications

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `use_keywords` | `bool` |  | true | Whether the search should use the keywords defined in the .desktop file. |

### Examples

_Basic app launcher_

```json
{
    "name": "App Launcher",
    "alias": "app",
    "type": "apps",
    "args": {
        "use_keywords": false
    },
    "priority": 4,
    "home": "Home"
}
```

---

## Process Launcher

`type = process`

Searches and terminates processes from within Sherlock.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `max_results` | `usize` |  | 50 | The maximum number of results to show in the process search. |
| `show_tile` | `bool` |  | false | Wheather a tile should be displayed of the user only wants the alias-based execution. |

### Inner Functions

| Name | Identifier | Description |
|---|---|---|
| Quit | `inner.quit` | Quit the current process |

### Examples

_Basic process terminator_

```json
{
    "name": "Processes",
    "type": "process",
    "alias": "kill",
    "args": {},
    "priority": 1,
    "home": "Home",
    "shortcut": false,
    "exit": false
}
```

---

## Script Launcher

`type = script`

Executes commands either on keypress (async) or on return. The results will be displayed within Sherlock.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `async` | `bool` |  | true | If set to true, will run the script on every keypress. If set to false, will wait for the execution of the `inner.run` command. |
| `exec` | `command` |  | false | Wheather a tile should be displayed of the user only wants the alias-based execution. |
| `exec` | `string` |  |  | The arguments to the command. Will replace `{keyword}` with the actual contents of the search bar. |

### Inner Functions

| Name | Identifier | Description |
|---|---|---|
| Run | `inner.run` | Run the current script. (Required if `async = false`) |

### Examples

_Basic process terminator_

```json
{
    "name": "Wikipedia Search",
    "alias": "wiki",
    "type": "script",
    "args": {
        "icon": "wikipedia",
        "exec": "sherlock-wiki",
        "exec-args": "'{keyword}'"
    },
    "priority": 0,
    "shortcut": false
}
```

---

## Theme Picker

`type = theme`

Preview and select Sherlock themes.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `path` | `path` |  | ~/.config/sherlock/themes/ | The path to the Sherlock themes directory. |

### Examples

_Basic process terminator_

```json
{
    "name": "Theme Picker",
    "type": "theme",
    "alias": "themes",
    "priority": 0,
    "exit": false
}
```

---

## Timer Launcher

`type = timer`

Start and run up to four timers concurrently. Each timer can have a unique action to be run at completion.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `exec` | `command` |  |  | The command to execute on timer completion. |

### Inner Functions

| Name | Identifier | Description |
|---|---|---|
| Toggle | `inner.toggle` | Toggle all timers |
| Reset | `inner.reset` | Reset all timers |

### Examples

_Basic process terminator_

```json
{
    "name": "Timer",
    "type": "timer",
    "args": {
        "exec": "notify-send \"hello\""
    },
    "priority": 1,
    "shortcut": false
}
```

---

## Translator

`type = translator`

Translate your queries into other languages.

### Examples

_Basic translator_

```json
{
    "name": "Translator",
    "alias": "trans",
    "type": "translator",
    "args": {},
    "on_return": "inner.run",
    "exit": false,
    "priority": 1,
    "shortcut": false
}
```

---

## Weather Launcher

`type = weather`

Display the weather and time in Sherlock.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `location` | `string` | ✓ | — | The location for the weather. |
| `update_interval` | `u64` | ✓ | — | The time in minutes after which to invalidate the cached weather condition. |
| `icon_theme` | `string` |  | — | The weather icon theme. Can be either `None` or `Sherlock` |
| `show_datetime` | `bool` |  | false | Whether to show a tile with the current date and time. |

### Examples

_Basic weather launcher_

```json
{
    "name": "Weather",
    "type": "weather",
    "args": {
        "location": "berlin",
        "update_interval": 120,
        "show_datetime": true,
        "icon_theme": "Sherlock"
    },
    "actions": [
        {
            "name": "Show in Web",
            "exec": "https://www.wttr.in/berlin",
            "icon": "sherlock-link",
            "method": "web_launcher"
        }
    ],
    "priority": 1,
    "home": "OnlyHome",
    "shortcut": false,
    "spawn_focus": false
}
```

---

## Web Launcher

`type = web`

Seach the current query in the specified engine using the specified browser.

### Args

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `search_engine` | `string` | ✓ | — | The search engine used for the query. |
| `browser` | `u64` |  | Default Browser | The browser in which to open the query. |
| `display_name` | `string` |  | — | The display name for this tile, replacing `{keyword}` with the actual contents of the search bar. |

### Examples

_Basic web launcher_

```json
{
    "name": "Web Search",
    "alias": "gg",
    "type": "web",
    "args": {
        "search_engine": "google",
        "icon": "google",
        "display_name": "Google Search {keyword}"
    },
    "home": "Persist",
    "priority": 100
}
```

---