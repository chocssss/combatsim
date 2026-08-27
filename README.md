## Building

Requires a recent stable Rust toolchain (install via [rustup](https://rustup.rs/) if needed).

```bash
cargo build --release
```

The binary is produced at `target/release/combatsim`.

## Usage

The simulator reads a player export (JSON) from stdin or a file, and a target zone:

```bash
./target/release/combatsim --input player.json --zone /actions/combat/sorcerers_tower --tier 0 --hours 24 --runs 2 --simple
```

### Options

| Flag | Description |
|---|---|
| `--zone ZONE` | Zone hrid, e.g. `/actions/combat/sorcerers_tower` (required unless `--all-zones` or `--optimize`) |
| `--tier N` | Difficulty tier 0-5 (default: 0) |
| `--hours N` | Simulation hours (default: 24) |
| `--input FILE` | Read player JSON from FILE instead of stdin |
| `--runs N` | Number of parallel simulation runs to average (default: 1) |
| `--market` | Fetch live market prices from milkywayidle.com for loot values |
| `--all-zones` | Run all multi-monster/boss zones at tiers 0-4, sorted by XP/hr |
| `--optimize PLAYER` | Optimise equipment/skills/abilities for the named player (add `--market` for accurate prices) |
| `--moo-pass` | Apply Moo Pass XP bonus |
| `--com-exp N` | Community XP buff level |
| `--com-drop N` | Community drop buff level |
| `--seal NAME` | Apply a seal buff (repeatable). Names: `seal_of_wisdom`, `seal_of_damage`, `seal_of_attack_speed`, `seal_of_cast_speed`, `seal_of_critical_rate`, `seal_of_combat_drop`, `seal_of_rare_find` |
| `--guild N` | Guild mode (N is guild level, 100-300). Disables consumables and grants a flat +3% HP/MP regen buff |
| `--list-zones` | List all available combat zones and exit |
| `--pretty` | Pretty-print JSON output |
| `--simple` | Human-readable summary (DPS, XP/hr, encounters, mana, deaths, loot) |

### Examples

List available zones:
```bash
./target/release/combatsim --list-zones
```

Run a 48-hour simulation with a readable summary:
```bash
cat export.json | ./target/release/combatsim --zone /actions/combat/sorcerers_tower --tier 2 --hours 48 --simple
```

Compare all zones for the best XP/hr:
```bash
cat export.json | ./target/release/combatsim --all-zones --market
```

Optimize a player's loadout:
```bash
cat export.json | ./target/release/combatsim --optimize player1 --market
```

## Input format

The simulator expects the player export JSON produced from the game client (a single player object, or multiple players for a party simulation). Game data (items, abilities, monsters, etc.) is bundled under `src/data/`.

## Building

Requires a recent stable Rust toolchain (install via [rustup](https://rustup.rs/) if needed).

```bash
cargo build --release
```

The binary is produced at `target/release/combatsim`.

## Usage

The simulator reads a player export (JSON) from stdin or a file, and a target zone:

```bash
./target/release/combatsim --input player.json --zone /actions/combat/sorcerers_tower --tier 0 --hours 24 --runs 2 --simple
```

### Options

| Flag | Description |
|---|---|
| `--zone ZONE` | Zone hrid, e.g. `/actions/combat/sorcerers_tower` (required unless `--all-zones` or `--optimize`) |
| `--tier N` | Difficulty tier 0-4 (default: 0) |
| `--hours N` | Simulation hours (default: 24) |
| `--input FILE` | Read player JSON from FILE instead of stdin |
| `--runs N` | Number of parallel simulation runs to average (default: 1) |
| `--market` | Fetch live market prices from milkywayidle.com for loot values |
| `--all-zones` | Run all multi-monster/boss zones at tiers 0-4, sorted by XP/hr |
| `--optimize PLAYER` | Optimise equipment/skills/abilities for the named player (add `--market` for accurate prices) |
| `--moo-pass` | Apply Moo Pass XP bonus |
| `--com-exp N` | Community XP buff level |
| `--com-drop N` | Community drop buff level |
| `--seal NAME` | Apply a seal buff (repeatable). Names: `seal_of_wisdom`, `seal_of_damage`, `seal_of_attack_speed`, `seal_of_cast_speed`, `seal_of_critical_rate`, `seal_of_combat_drop`, `seal_of_rare_find` |
| `--guild N` | Guild mode (N is guild level, 100-300). Disables consumables and grants a flat +3% HP/MP regen buff |
| `--list-zones` | List all available combat zones and exit |
| `--pretty` | Pretty-print JSON output |
| `--simple` | Human-readable summary (DPS, XP/hr, encounters, mana, deaths, loot) |

### Examples

List available zones:
```bash
./target/release/combatsim --list-zones
```

Run a 48-hour simulation with a readable summary:
```bash
cat export.json | ./target/release/combatsim --zone /actions/combat/sorcerers_tower --tier 2 --hours 48 --simple
```

Compare all zones for the best XP/hr:
```bash
cat export.json | ./target/release/combatsim --all-zones --market
```

Optimize a player's loadout:
```bash
cat export.json | ./target/release/combatsim --optimize player1 --market
```

## Input format

The simulator expects the player export JSON produced from the game client (a single player object, or multiple players for a party simulation). Game data (items, abilities, monsters, etc.) is bundled under `src/data/`.
