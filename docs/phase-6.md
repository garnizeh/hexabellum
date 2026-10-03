# Phase 6 Technical Spec  

## Economy, Levels, and Minimal Items Slice

---

## 1. Phase 6 Goal

> **Add the first MOBA progression layer: gold, XP, hero levels, and a minimal passive item system — all validated by the server and integrated into the 5v5 player-per-hero architecture from Phase 5.**

Phase 5 proved the multiplayer player model.  
Phase 6 gives players a reason to fight, farm, and grow stronger over time.

---

## 2. Why This Slice Matters

A MOBA needs progression pressure:

- killing enemies should matter
- minion waves should matter
- neutral camps should matter
- destroying structures should matter
- heroes should become stronger over time
- players should make build decisions

Phase 6 introduces these systems in the simplest possible way.

---

## 3. Phase 6 Definition of Done

Phase 6 is complete when:

- [ ] Each hero has gold.
- [ ] Each hero has XP and level.
- [ ] Gold is granted from rounds, kills, neutrals, and structure destruction.
- [ ] XP is granted from kills, neutrals, and structure destruction.
- [ ] Heroes can level up.
- [ ] Level-ups improve hero stats.
- [ ] A small set of passive items exists.
- [ ] Heroes can buy items during planning.
- [ ] Items occupy limited inventory slots.
- [ ] Items modify hero stats.
- [ ] The server validates all purchases.
- [ ] AI heroes can buy items.
- [ ] Client displays gold, XP, level, items, and shop.
- [ ] Economy events are visible in the UI.
- [ ] The game remains stable in 5v5 mode.

---

# 4. Phase 6 Scope

## Included

### Economy
- gold per hero
- passive gold income
- kill rewards
- neutral rewards
- structure destruction rewards

### Progression
- XP per hero
- hero levels
- level-up stat growth
- max level for MVP

### Items
- passive items only
- fixed item costs
- limited item slots
- no duplicates
- stat modifiers

### Shop
- planning-phase shopping
- simplified “field shop” for the slice
- server-validated purchases
- AI shopping

### UI
- gold display
- XP / level display
- item slots
- shop panel
- reward notifications
- level-up notification

---

## Excluded

Phase 6 does **not** include:

- selling items
- item recipes
- consumables
- active items
- inventory management beyond slot limit
- gold loss on death
- XP sharing radius
- base-only shop
- multiple shops
- comeback mechanics
- matchmaking
- ranked progression
- cosmetics
- persistence
- advanced objectives

---

# 5. Simplification Decision: Field Shop

For this slice, use a simplified shop model:

> **Heroes may buy items during planning from anywhere.**

This is not the final MOBA shop model, but it is the best Phase 6 vertical slice choice because it lets us validate:
- gold
- XP
- leveling
- item stats
- purchase validation
- AI shopping
- UI flow

without adding base positioning complexity.

A proper base shop can be added in Phase 7.

---

# 6. Economy Design

---

## 6.1 Gold Ownership

Gold is owned per hero.

This works well because:
- Phase 5 uses one hero per player
- each player can have their own build
- AI heroes can also use gold
- future item system remains unit-centric

---

## 6.2 Starting Gold

Suggested:

```rust
starting_gold: 50
```

This allows early first-item pressure after one or two rounds.

---

## 6.3 Passive Income

At the start of each planning round:

```rust
+6 gold per alive hero
```

This keeps the game moving even without last hits.

---

## 6.4 Kill Rewards

### Hero Kill
```rust
+30 gold to killer hero
+30 XP to killer hero
```

### Minion Kill
```rust
+10 gold to killer hero
+10 XP to killer hero
```

### Neutral Guardian Kill
```rust
+40 gold to killer hero
+25 XP to killer hero
```

This stacks with the Phase 4 neutral buff.

---

## 6.5 Structure Rewards

When an enemy structure is destroyed:

### Tower Destroyed
```rust
+25 gold to each alive enemy hero
+20 XP to each alive enemy hero
```

### Spawner Destroyed
```rust
+30 gold to each alive enemy hero
+25 XP to each alive enemy hero
```

This makes objectives valuable even if the final blow comes from a minion.

---

## 6.6 Killer Attribution

Use `last_attacker` on damaged units.

Rules:
- when a unit takes damage, store the attacker ID
- if the unit dies, reward the last attacker if:
  - attacker is a hero
  - attacker is still alive
- if attacker is not a hero or is dead:
  - no individual kill reward
  - structure team rewards still apply where relevant

This is simple and deterministic.

---

# 7. XP and Level Design

---

## 7.1 Level Range

For Phase 6:

```rust
min_level: 1
max_level: 5
```

This is enough to demonstrate progression without opening full balance complexity.

---

## 7.2 XP Thresholds

| Level | Total XP Required |
|---:|---:|
| 1 | 0 |
| 2 | 50 |
| 3 | 120 |
| 4 | 220 |
| 5 | 350 |

---

## 7.3 Level-Up Bonuses

When a hero levels up:

```rust
+12 max HP
+3 attack damage
+1 max energy
+12 immediate healing
```

AP is intentionally not increased by default to avoid action economy imbalance.

---

## 7.4 XP Sources Summary

| Source | XP |
|---|---:|
| minion kill | 10 |
| hero kill | 30 |
| neutral kill | 25 |
| tower destroyed | 20 to each alive enemy hero |
| spawner destroyed | 25 to each alive enemy hero |

---

# 8. Item System

Phase 6 uses only passive items.

---

## 8.1 Item Definition

```rust
pub type ItemDefId = String;

pub struct ItemDef {
    pub id: ItemDefId,
    pub name: String,
    pub cost: u32,
    pub modifiers: Vec<StatModifier>,
    pub description: String,
}
```

---

## 8.2 Item Slot Rules

```rust
max_item_slots: 3
allow_duplicates: false
allow_sell: false
```

A hero can own at most:
- 3 items
- one copy of each item

---

## 8.3 Phase 6 Item Set

Keep the item set tiny.

### Longblade
```rust
id: "longblade"
cost: 100
modifiers: [
    AttackDamage +6
]
```

### Plate Armor
```rust
id: "plate_armor"
cost: 120
modifiers: [
    MaxHealth +35
]
```

Special rule:
- when purchased, also increase current HP by 35

### Scout Lens
```rust
id: "scout_lens"
cost: 80
modifiers: [
    VisionRange +1
]
```

### Focus Charm
```rust
id: "focus_charm"
cost: 100
modifiers: [
    EnergyRegen +1
]
```

---

## 8.4 Why No Consumables Yet?

Consumables require:
- charges
- activation actions
- stack rules
- inventory edge cases

Those are better suited for a later slice.

---

# 9. Shop Design

---

## 9.1 Shop Availability

For Phase 6:

- shop is available during planning
- shop can be used from anywhere
- hero must be alive
- player must control the hero
- purchase is instant and server-validated

---

## 9.2 Purchase Rules

A purchase is valid if:
- match is in planning phase
- hero is alive
- player controls hero
- hero has enough gold
- hero has free item slot
- hero does not already own that item

---

## 9.3 Purchase Effects

When purchase succeeds:
1. subtract gold
2. add item to hero inventory
3. apply stat modifiers
4. emit event
5. update snapshot / economy UI

---

## 9.4 Purchases and Orders

Purchases:
- do not consume AP
- can happen before or after submitting orders
- affect resolution state immediately

If an item changes stats, already submitted orders are not retroactively revalidated until resolution.

---

# 10. Core Simulation Changes

---

## 10.1 Unit Additions

Add these fields to hero units:

```rust
pub struct Unit {
    // Existing fields
    pub id: UnitId,
    pub kind: UnitKind,
    pub team: TeamId,
    pub pos: HexCoord,
    pub hp: u32,
    pub max_hp: u32,
    pub ap: u32,
    pub max_ap: u32,
    pub initiative: u32,
    pub attack_damage: u32,
    pub attack_range: u32,
    pub vision_range: u32,
    pub energy: u32,
    pub max_energy: u32,
    pub energy_regen: u32,

    // Phase 6 additions
    pub gold: u32,
    pub xp: u32,
    pub level: u32,
    pub items: Vec<ItemDefId>,
}
```

Minions, towers, spawners, and neutrals can keep default values:
```rust
gold: 0,
xp: 0,
level: 0,
items: []
```

---

## 10.2 Stat Application Strategy

For Phase 6, use direct mutation for permanent progression:

- level-ups directly increase base stats
- item purchases directly increase base stats
- temporary statuses still use the Phase 4 status system

This is simpler than a full layered stat system and is enough for Phase 6.

---

## 10.3 Level-Up Function

```rust
pub fn apply_level_up(unit: &mut Unit) {
    unit.level += 1;
    unit.max_hp += 12;
    unit.hp = (unit.hp + 12).min(unit.max_hp);
    unit.attack_damage += 3;
    unit.max_energy += 1;
}
```

---

## 10.4 XP Grant Function

```rust
pub fn grant_xp(unit: &mut Unit, amount: u32) {
    if unit.level >= MAX_LEVEL {
        return;
    }

    unit.xp += amount;

    while unit.level < MAX_LEVEL && unit.xp >= xp_threshold(unit.level + 1) {
        apply_level_up(unit);
    }
}
```

---

# 11. Reward Pipeline

Phase 6 needs a centralized reward service.

---

## 11.1 Reward Reasons

```rust
pub enum RewardReason {
    PassiveIncome,
    HeroKill,
    MinionKill,
    NeutralKill,
    TowerDestroyed,
    SpawnerDestroyed,
}
```

---

## 11.2 Reward Event

```rust
pub struct RewardGranted {
    pub unit_id: UnitId,
    pub gold: u32,
    pub xp: u32,
    pub reason: RewardReason,
}
```

This can be emitted either:
- inside round events
- or as a planning-phase economy event for passive income / purchases

---

## 11.3 Where Rewards Are Applied

### At Round Start
- passive income

### During Resolution
- kills
- neutral deaths
- structure destruction

---

# 12. Server Changes

---

## 12.1 New Server Responsibilities

The server must now handle:
- gold ledger
- XP ledger
- level state
- item ownership
- shop validation
- reward distribution
- AI purchases

---

## 12.2 MatchActor Additions

```rust
pub struct MatchActor {
    // Existing fields
    item_catalog: Vec<ItemDef>,
}
```

No separate economy service is needed yet. The battle session can own economy state.

---

## 12.3 New Match Command

```rust
pub enum MatchCommand {
    // Existing commands...

    BuyItem {
        player_id: PlayerId,
        item_id: ItemDefId,
    },
}
```

---

## 12.4 Buy Item Handler Rules

When `BuyItem` is received:

1. verify match phase is planning
2. verify player exists and controls a hero
3. verify hero is alive
4. verify item exists
5. verify hero does not already own item
6. verify hero has free slot
7. verify hero has enough gold
8. subtract gold
9. add item
10. apply modifiers
11. emit event
12. send updated state to relevant clients

---

# 13. AI Shopping

AI heroes should participate in the economy.

---

## 13.1 When AI Shops

At the start of each planning round, after passive income:

- server evaluates AI hero purchases
- purchases are applied immediately
- resulting state is included in the planning snapshot

---

## 13.2 Simple AI Buy Priority

Use deterministic priority:

1. if gold >= 120 and no `plate_armor`, buy `plate_armor`
2. else if gold >= 100 and no `longblade`, buy `longblade`
3. else if gold >= 100 and no `focus_charm`, buy `focus_charm`
4. else if gold >= 80 and no `scout_lens`, buy `scout_lens`

This is simple, stable, and good enough for testing.

---

## 13.3 Future Improvement

Later AI can choose items based on:
- hero role
- enemy composition
- HP state
- vision needs
- energy usage

Not needed for Phase 6.

---

# 14. Protocol Updates

---

## 14.1 Client Message Additions

```rust
BuyItem {
    item_id: ItemDefId,
}
```

---

## 14.2 Server Message Additions

```rust
PurchaseResolved {
    unit_id: UnitId,
    item_id: ItemDefId,
    gold_remaining: u32,
    success: bool,
    error: Option<String>,
}

EconomyUpdated {
    unit_id: UnitId,
    gold: u32,
    xp: u32,
    level: u32,
    items: Vec<ItemDefId>,
}

LevelUpOccurred {
    unit_id: UnitId,
    new_level: u32,
}
```

---

## 14.3 Snapshot Additions

Add to `SnapshotDto`:

```rust
pub controlled_hero_economy: Option<HeroEconomyDto>,
pub allied_hero_economy: Vec<HeroEconomyDto>,
pub shop_items: Vec<ItemDto>,
pub can_shop: bool,
```

Where:

```rust
pub struct HeroEconomyDto {
    pub unit_id: UnitId,
    pub hero_def_id: HeroDefId,
    pub gold: u32,
    pub xp: u32,
    pub level: u32,
    pub items: Vec<ItemDefId>,
}
```

---

## 14.4 Item DTO

```rust
pub struct ItemDto {
    pub id: ItemDefId,
    pub name: String,
    pub cost: u32,
    pub description: String,
    pub modifiers: Vec<StatModifierDto>,
}
```

---

# 15. Client UI Updates

Phase 6 needs a clear economy UI.

---

## 15.1 Controlled Hero Panel

Show:
- gold
- level
- XP bar
- item slots
- ability bar
- HP / AP / energy

---

## 15.2 Team Roster Additions

For allied heroes show:
- level
- item count or item icons
- alive state
- connection state

Do not show enemy gold or items unless visible and desired later.

For Phase 6:
- show allied economy fully
- hide enemy economy

---

## 15.3 Shop Panel

The shop panel should show:
- item name
- item cost
- item description
- whether hero can afford it
- whether slot is available
- whether item is already owned
- buy button

---

## 15.4 Reward Notifications

Show lightweight toast messages:

```text
+10 gold — minion kill
+30 XP — hero kill
Level Up! Vanguard reached level 3
Purchased Longblade
```

---

## 15.5 Level-Up Feedback

Use:
- floating text
- brief hero glow
- roster level badge update

Keep it simple.

---

# 16. Economy Privacy Rules

Economy information should be treated carefully.

## Visible to Team
- allied hero gold
- allied hero XP / level
- allied hero items

## Hidden from Enemy
- enemy gold
- enemy XP
- enemy items

For Phase 6, enemy level may remain hidden unless the enemy hero is visible.

Simplest rule:
- include economy only for allied heroes
- omit enemy economy entirely

---

# 17. Interaction With Existing Systems

---

## 17.1 Phase 4 Abilities
Ability costs remain unchanged.

Leveling and items may increase:
- damage
- HP
- vision
- energy regen

but do not directly reduce cooldowns in Phase 6.

---

## 17.2 Phase 4 Neutral Buff
Neutral guardians now grant:
- Phase 4 damage buff
- Phase 6 gold reward
- Phase 6 XP reward

This makes neutrals much more valuable.

---

## 17.3 Phase 5 Roster
Roster now includes:
- level
- item count
- maybe small item icons

---

## 17.4 Phase 5 AI
AI now needs:
- shopping behavior
- slightly better target value awareness

But no advanced farming behavior is required yet.

---

# 18. Determinism and Economy

Economy must remain deterministic.

## Rules
- rewards are granted in fixed event order
- passive income happens at a fixed point in round start
- level-ups are processed immediately after XP grant
- AI purchases happen in deterministic unit ID order
- no time-based rewards outside round boundaries

---

# 19. Suggested Configuration

```rust
pub struct EconomyConfig {
    pub starting_gold: u32,
    pub passive_income_per_round: u32,
    pub hero_kill_gold: u32,
    pub minion_kill_gold: u32,
    pub neutral_kill_gold: u32,
    pub tower_destroy_gold_per_hero: u32,
    pub spawner_destroy_gold_per_hero: u32,

    pub hero_kill_xp: u32,
    pub minion_kill_xp: u32,
    pub neutral_kill_xp: u32,
    pub tower_destroy_xp_per_hero: u32,
    pub spawner_destroy_xp_per_hero: u32,

    pub max_level: u32,
    pub max_item_slots: u32,
    pub allow_duplicate_items: bool,
}
```

Default:

```rust
EconomyConfig {
    starting_gold: 50,
    passive_income_per_round: 6,
    hero_kill_gold: 30,
    minion_kill_gold: 10,
    neutral_kill_gold: 40,
    tower_destroy_gold_per_hero: 25,
    spawner_destroy_gold_per_hero: 30,

    hero_kill_xp: 30,
    minion_kill_xp: 10,
    neutral_kill_xp: 25,
    tower_destroy_xp_per_hero: 20,
    spawner_destroy_xp_per_hero: 25,

    max_level: 5,
    max_item_slots: 3,
    allow_duplicate_items: false,
}
```

---

# 20. Implementation Order

To keep Phase 6 safe, implement in this order:

---

## Step 1 — Economy Ledger
Add:
- gold field
- XP field
- level field
- reward reasons
- reward events

---

## Step 2 — XP and Levels
Add:
- XP thresholds
- level-up logic
- level-up events

---

## Step 3 — Kill Attribution Rewards
Add:
- last attacker tracking
- hero kill rewards
- minion kill rewards
- neutral kill rewards

---

## Step 4 — Structure Rewards
Add:
- tower destruction rewards
- spawner destruction rewards

---

## Step 5 — Item Definitions
Add:
- item catalog
- item modifiers
- slot rules

---

## Step 6 — Purchase System
Add:
- buy command
- validation
- stat application
- purchase events

---

## Step 7 — AI Shopping
Add:
- deterministic AI item selection
- AI purchases at round start

---

## Step 8 — Client UI
Add:
- gold display
- XP bar
- level display
- shop panel
- item slots
- notifications

---

# 21. Acceptance Criteria

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Heroes start with configured gold | ☐ |
| 2 | Heroes receive passive income each round | ☐ |
| 3 | Hero kills grant gold and XP | ☐ |
| 4 | Minion kills grant gold and XP | ☐ |
| 5 | Neutral kills grant gold and XP | ☐ |
| 6 | Tower destruction grants team rewards | ☐ |
| 7 | Spawner destruction grants team rewards | ☐ |
| 8 | XP thresholds trigger level-ups | ☐ |
| 9 | Level-ups increase stats | ☐ |
| 10 | Level-ups are emitted as events | ☐ |
| 11 | Items exist in server catalog | ☐ |
| 12 | Heroes can buy items during planning | ☐ |
| 13 | Purchases require enough gold | ☐ |
| 14 | Purchases require free item slots | ☐ |
| 15 | Duplicate items are rejected | ☐ |
| 16 | Item modifiers apply correctly | ☐ |
| 17 | AI heroes buy items deterministically | ☐ |
| 18 | Client displays gold | ☐ |
| 19 | Client displays XP and level | ☐ |
| 20 | Client displays item slots | ☐ |
| 21 | Client displays shop | ☐ |
| 22 | Purchase failures show clear errors | ☐ |
| 23 | Allied economy is visible to team | ☐ |
| 24 | Enemy economy is hidden | ☐ |
| 25 | 5v5 matches remain stable with economy enabled | ☐ |

---

# 22. Risks and Mitigations

---

## Risk 1: Economy Snowballs Too Fast
### Mitigation
Use conservative rewards and short max level for now.

---

## Risk 2: Killer Attribution Is Ambiguous
### Mitigation
Use last attacker only and keep rules simple.

---

## Risk 3: Purchases During Planning Complicate State Sync
### Mitigation
Apply purchases immediately on server and broadcast economy updates.

---

## Risk 4: Item Stats Break Balance
### Mitigation
Use small values and only four items in Phase 6.

---

## Risk 5: AI Becomes Too Passive or Too Greedy
### Mitigation
Use fixed deterministic buy priority and simple combat behavior.

---

# 23. Out of Scope

Again, Phase 6 does **not** include:

- consumables
- active items
- item recipes
- selling
- base shop
- gold loss on death
- XP sharing radius
- multiple item builds
- persistent account progression
- matchmaking
- ranked economy
- cosmetics

---

# 24. Phase 7 Preview

After Phase 6, the next slice should likely focus on:

## Phase 7 — Objectives, Base Shop, and Win Conditions

Possible contents:
- base-only shopping
- tower destruction as primary win condition
- multiple lanes
- inhibitor / core structures
- stronger neutral objectives
- comeback mechanics
- item actives or consumables
- richer team objectives

Phase 6 gives us the progression foundation needed for that slice.

---

# 25. Final Recommendation

Phase 6 should be:

> **A server-authoritative economy and progression slice with gold, XP, levels, and minimal passive items.**

This is the right next step because it turns battles into a growing strategic conflict instead of a static tactical fight.

If you want, the next document can be:

**“Phase 6 implementation task breakdown”**  
with concrete core/server/protocol/client tasks in execution order.