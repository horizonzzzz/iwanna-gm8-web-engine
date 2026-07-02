# GM8 Motion Variable Bidirectional Sync Design

> **Status:** Historical / implemented
> **Date:** 2026-06-16
> **Scope:** `crates/iwm-runtime-core/`, `crates/iwm-runtime-model/`
>
> This design has been implemented for the current runtime-core slice. The
> active source of truth is now the code and tests around
> `RuntimeInstance::set_speed`, `RuntimeInstance::set_direction`,
> `RuntimeInstance::set_hspeed`, `RuntimeInstance::set_vspeed`,
> `RuntimeInstance::set_hvspeed`, `apply_gm_motion_vars()`, and the
> `movement_math` / movement tests.

## Problem

Blood splash physics in the runtime do not match the original GM8 exe. The root cause is that `apply_gm_motion_vars()` in `movement.rs` only reads `speed`/`direction` from the vars map when `hspeed == 0 && vspeed == 0`, and never syncs back. In GM8, every assignment to `speed`, `direction`, `hspeed`, or `vspeed` immediately updates the complementary pair.

Three specific gaps:

1. **No bidirectional sync** — setting `b.direction = i; b.speed = 4` writes to vars but does not update `hspeed`/`vspeed` until the next frame, and only if both are still zero.
2. **No friction** — GM8 applies friction (pulling `speed` toward 0) before gravity every frame. The runtime ignores friction entirely.
3. **No 0.0001 rounding tolerance** — GM8 rounds `hspeed`/`vspeed` to the nearest integer when the difference is less than 0.0001, which matters for cardinal directions (0°, 90°, 180°, 270°).

## Reference

OpenGMK implementation in `vendor/OpenGMK/gm8emulator/src/instance.rs`:

- `set_speed()` / `set_direction()` → call `update_hvspeed()` (sync to hspeed/vspeed with 0.0001 tolerance)
- `set_hspeed()` / `set_vspeed()` → call `update_speed_direction()` (sync to speed/direction)
- `set_hvspeed()` → update both, then sync speed/direction

OpenGMK per-frame order in `vendor/OpenGMK/gm8emulator/src/game/movement.rs`:

1. Apply friction (pull speed toward 0 via `set_speed`)
2. Apply gravity (add to hspeed/vspeed via `set_hvspeed`)
3. Update positions (`x += hspeed; y += vspeed`)

## Design

### 1. Instance-level sync methods on `RuntimeInstance`

Add methods to `RuntimeInstance` in `types.rs`:

```rust
impl RuntimeInstance {
    pub fn set_speed(&mut self, speed: f64) {
        self.vars.insert("speed".into(), RuntimeValue::Number(speed));
        self.sync_hvspeed_from_speed_direction();
    }

    pub fn set_direction(&mut self, direction: f64) {
        let normalized = direction.rem_euclid(360.0);
        self.vars.insert("direction".into(), RuntimeValue::Number(normalized));
        self.sync_hvspeed_from_speed_direction();
    }

    pub fn set_hspeed(&mut self, hspeed: f64) {
        if self.hspeed != hspeed {
            self.hspeed = hspeed;
            self.sync_speed_direction_from_hvspeed();
        }
    }

    pub fn set_vspeed(&mut self, vspeed: f64) {
        if self.vspeed != vspeed {
            self.vspeed = vspeed;
            self.sync_speed_direction_from_hvspeed();
        }
    }

    pub fn set_hvspeed(&mut self, hspeed: f64, vspeed: f64) {
        if self.hspeed != hspeed || self.vspeed != vspeed {
            self.hspeed = hspeed;
            self.vspeed = vspeed;
            self.sync_speed_direction_from_hvspeed();
        }
    }
}
```

### 2. Sync helpers with 0.0001 rounding tolerance

```rust
const GM_ROUND_TOLERANCE: f64 = 0.0001;

impl RuntimeInstance {
    fn sync_hvspeed_from_speed_direction(&mut self) {
        let speed = self.vars.get("speed").and_then(as_number).unwrap_or(0.0);
        let direction = self.vars.get("direction").and_then(as_number).unwrap_or(0.0);
        let radians = direction.to_radians();
        let raw_hspeed = radians.cos() * speed;
        let raw_vspeed = -radians.sin() * speed;
        self.hspeed = gm_round(raw_hspeed);
        self.vspeed = gm_round(raw_vspeed);
    }

    fn sync_speed_direction_from_hvspeed(&mut self) {
        let direction = (-self.vspeed).atan2(self.hspeed).to_degrees().rem_euclid(360.0);
        let speed = (self.hspeed * self.hspeed + self.vspeed * self.vspeed).sqrt();
        self.vars.insert("direction".into(), RuntimeValue::Number(gm_round(direction)));
        self.vars.insert("speed".into(), RuntimeValue::Number(speed));
    }
}

fn gm_round(value: f64) -> f64 {
    let rounded = value.round();
    if (rounded - value).abs() < GM_ROUND_TOLERANCE {
        rounded
    } else {
        value
    }
}
```

### 3. Lowered logic assignment path changes

In `logic/statement.rs`, when a lowered assignment targets `speed`, `direction`, `hspeed`, or `vspeed` (either directly or through member access like `b.speed = 4`), call the sync methods instead of writing to the vars map directly.

For `hspeed`/`vspeed` direct assignments on the instance, call `set_hspeed()`/`set_vspeed()` instead of writing the field directly.

For `speed`/`direction` assignments, call `set_speed()`/`set_direction()` instead of inserting into vars.

### 4. Rewrite `apply_gm_motion_vars()`

Replace the current implementation with the GM8-faithful order:

```rust
fn apply_gm_motion_vars(instance: &mut RuntimeInstance) {
    // 1. Apply friction (pull speed toward 0)
    if let Some(friction) = instance.vars.get("friction").and_then(as_number) {
        if friction != 0.0 {
            let speed = instance.vars.get("speed").and_then(as_number).unwrap_or(0.0);
            if speed >= 0.0 {
                if friction > speed {
                    instance.set_speed(0.0);
                } else if speed != 0.0 {
                    instance.set_speed(speed - friction);
                }
            } else {
                if friction > -speed {
                    instance.set_speed(0.0);
                } else if speed != 0.0 {
                    instance.set_speed(speed + friction);
                }
            }
        }
    }

    // 2. Apply gravity
    if let Some(gravity) = instance.vars.get("gravity").and_then(as_number) {
        if gravity != 0.0 {
            let gravity_direction = instance
                .vars
                .get("gravity_direction")
                .and_then(as_number)
                .unwrap_or(270.0);
            let radians = gravity_direction.to_radians();
            let new_hspeed = instance.hspeed + radians.cos() * gravity;
            let new_vspeed = instance.vspeed - radians.sin() * gravity;
            instance.set_hvspeed(new_hspeed, new_vspeed);
        }
    }
}
```

### 5. Friction field

Friction is read from `instance.vars` (like `gravity` and `gravity_direction` already are). No new field on `RuntimeInstance` is needed — the vars map already holds it if the GML sets it.

### 6. Player movement interaction

Player movement in `step_player()` uses hardcoded `hspeed`/`vspeed` fields directly and does not go through `apply_gm_motion_vars()`. The player path should remain unchanged — the sync methods are for non-player instances and for lowered logic assignments.

If a lowered player step script assigns `speed` or `direction`, the assignment path will now correctly sync to `hspeed`/`vspeed`, which is the desired behavior.

## Testing

### Unit tests in `crates/iwm-runtime-core/src/tests/movement.rs`

1. `set_speed_syncs_hvspeed` — set speed=4, direction=0 → hspeed=4, vspeed=0
2. `set_direction_syncs_hvspeed` — set direction=90, speed=3 → hspeed=0, vspeed=-3
3. `set_hspeed_syncs_speed_direction` — set hspeed=3, vspeed=4 → speed=5, direction≈36.87°
4. `gm_round_tolerance` — direction=0, speed=3 → hspeed=3.0 (not 2.9999999)
5. `friction_pulls_speed_toward_zero` — speed=5, friction=1 → after apply: speed=4
6. `friction_does_not_overshoot_zero` — speed=0.5, friction=1 → after apply: speed=0
7. `gravity_applied_after_friction` — speed=4 direction=0, gravity=0.5 direction=270 → verify both effects
8. `blood2_direction_speed_assignment_syncs_immediately` — `b.direction=45; b.speed=6` → hspeed/vspeed correct immediately

### Integration test

Extend the existing `real_sample_death_feedback` test or add a new test that verifies blood2 particle trajectories match expected values after the sync fix.

## Scope

- In scope: sync methods, friction, rounding tolerance, assignment path changes, tests
- Out of scope: `move_bounce`, `motion_add`/`motion_set` GML functions, particle system, broader GM8 physics beyond the current IWanna-critical subset
