use super::*;
use crate::special::{BossSpec, Form, Shifter};
pub(super) fn parse(g: &GameData, d: &Document) -> Result<BossSpec> {
    let mut spec = BossSpec::default();
    for r in d.records_keyed("tag") {
        known(r, &["id"])?;
        spec.tags.push(text(r, "id")?.into());
    }
    if let Some(r) = d.records_keyed("shared_health").next() {
        known(r, &["id"])?;
        spec.shared_health = Some(text(r, "id")?.into());
    }
    if let Some(r) = d.records_keyed("shape_shifter").next() {
        known(
            r,
            &[
                "monster_class_ids",
                "monster_class_chances",
                "monster_class_valid_ranks",
                "round_frequency",
            ],
        )?;
        let ids = &r
            .param("monster_class_ids")
            .ok_or("missing shape-shifter IDs")?
            .values;
        let weights = &r
            .param("monster_class_chances")
            .ok_or("missing shape-shifter weights")?
            .values;
        let ranks = &r
            .param("monster_class_valid_ranks")
            .ok_or("missing shape-shifter ranks")?
            .values;
        if ids.len() != weights.len() || ids.len() != ranks.len() {
            return Err("shape-shifter vector length mismatch".into());
        }
        let mut forms = vec![];
        for ((id, weight), rank) in ids.iter().zip(weights).zip(ranks) {
            g.monster_document(&id.text)?;
            forms.push(Form {
                choice: crate::lifecycle::Choice {
                    class: id.text.clone(),
                    weight: scalar(&weight.text)?,
                    limit: None,
                },
                ranks: Target::parse(&rank.text)?.ranks,
            });
        }
        let frequency = integer(r, "round_frequency")?
            .try_into()
            .map_err(|_| "invalid shape frequency")?;
        spec.shifter = Some(Shifter { forms, frequency });
    }
    more(g, d, &mut spec)?;
    spec.validate()?;
    Ok(spec)
}
fn known(r: &Record, allowed: &[&str]) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    if r.params.iter().any(|p| !seen.insert(&p.name)) {
        return Err(format!("duplicate boss {} parameter", r.key));
    }
    if r.params.iter().any(|p| !allowed.contains(&p.name.as_str())) {
        return Err(format!("unknown boss {} field", r.key));
    }
    Ok(())
}

fn words(r: &Record, k: &str) -> Vec<String> {
    r.param(k)
        .map(|p| p.values.iter().map(|v| v.text.clone()).collect())
        .unwrap_or_default()
}
fn effects(g: &GameData, r: &Record, k: &str) -> Result<Vec<Effect>> {
    words(r, k).iter().map(|s| g.effect(s)).collect()
}
fn more(g: &GameData, d: &Document, s: &mut BossSpec) -> Result<()> {
    use crate::special::*;
    if let Some(r) = d.records_keyed("wave_spawning").next() {
        known(r, &["prefers_front"])?;
        s.wave_front = Some(flag(r, "prefers_front", false)?);
    }
    for key in [
        "shared_health",
        "shape_shifter",
        "captor_empty",
        "captor_full",
        "controller",
        "companion",
        "skill_reaction",
        "death_damage",
        "torchlight_modifier",
        "battle_backdrop",
        "audio_modifier",
        "tutorial",
        "mash_modifier",
    ] {
        if d.records_keyed(key).count() > 1 {
            return Err(format!("ambiguous boss record {key}"));
        }
    }
    for r in d.records_keyed("mode") {
        known(
            r,
            &[
                "id",
                "is_raid_default",
                "always_guard_actor_base_class_ids",
                "is_targetable",
                "keep_rounds_in_ranks",
            ],
        )?;
        let ids = words(r, "id");
        if r.param("id").is_none() || ids.len() > 1 {
            return Err("invalid mode ID".into());
        }
        s.modes.push(Mode {
            id: ids.first().cloned().unwrap_or_default(),
            initial: flag(r, "is_raid_default", false)?,
            guard_families: words(r, "always_guard_actor_base_class_ids"),
            targetable: flag(r, "is_targetable", true)?,
            keep_rank_clock: flag(r, "keep_rounds_in_ranks", false)?,
        });
    }
    if let Some(r) = d.records_keyed("battle_modifier").next() {
        known(
            r,
            &[
                "disable_stall_penalty",
                "accelerate_stall_penalty",
                "can_surprise",
                "can_be_surprised",
                "always_surprise",
                "always_be_surprised",
                "can_be_hit",
                "can_be_random_target",
                "is_valid_friendly_target",
                "can_be_guarded",
                "can_be_summon_rank",
                "living_other_enemy_buffs",
                "living_hero_buff_instance_ids",
                "can_be_damaged_directly",
                "disabled_act_out_combat_start_turn_types",
                "can_be_missed",
                "can_relieve_stress_from_killing_blow",
                "does_count_as_monster_size_for_monster_brain",
                "does_count_as_guardable_for_monster_brain",
                "does_count_towards_stall_penalty",
                "can_relieve_stress_from_crit",
            ],
        )?;
        let blocked = words(r, "disabled_act_out_combat_start_turn_types");
        if blocked.iter().any(|s| s != "retreat_from_combat") {
            return Err("unsupported disabled act out type".into());
        }
        s.restrictions = Restrictions {
            no_trinket_target: false,
            no_trinket_attacker: false,
            no_crit_effects: false,
            no_direct_damage: !flag(r, "can_be_damaged_directly", true)?,
            cannot_miss: !flag(r, "can_be_missed", true)?,
            no_kill_relief: !flag(r, "can_relieve_stress_from_killing_blow", true)?,
            no_crit_relief: !flag(r, "can_relieve_stress_from_crit", true)?,
            no_brain_size: !flag(r, "does_count_as_monster_size_for_monster_brain", true)?,
            no_brain_guard: !flag(r, "does_count_as_guardable_for_monster_brain", true)?,
            no_stall_count: !flag(r, "does_count_towards_stall_penalty", true)?,
            block_retreat_actout: !blocked.is_empty(),
        };
    }
    if let Some(r) = d.records_keyed("additional_effect").next() {
        known(r, &["is_valid_trinket_target", "is_valid_trinket_attacker"])?;
        s.restrictions.no_trinket_target = !flag(r, "is_valid_trinket_target", true)?;
        s.restrictions.no_trinket_attacker = !flag(r, "is_valid_trinket_attacker", true)?;
    }
    if let Some(r) = d.records_keyed("crit").next() {
        known(r, &["is_valid_effects_target"])?;
        s.restrictions.no_crit_effects = !flag(r, "is_valid_effects_target", true)?;
    }
    if let Some(r) = d.records_keyed("captor_empty").next() {
        known(
            r,
            &[
                "performing_monster_captor_base_class",
                "captor_full_monster_class",
                "capture_effects",
                "reset_hp",
                "count_captor_full_damage",
            ],
        )?;
        let full = text(r, "captor_full_monster_class")?.to_string();
        g.monster_document(&full)?;
        s.empty = Some(CaptorEmpty {
            performer: text(r, "performing_monster_captor_base_class")?.into(),
            full,
            effects: effects(g, r, "capture_effects")?,
            reset_hp: flag(r, "reset_hp", true)?,
            count_damage: flag(r, "count_captor_full_damage", false)?,
        });
    }
    if let Some(r) = d.records_keyed("captor_full").next() {
        known(
            r,
            &[
                "captor_empty_monster_class",
                "release_on_death",
                "release_on_prisoner_at_deaths_door",
                "per_turn_damage_percent",
                "switch_class_on_death",
                "release_on_prisoner_affliction",
                "release_effects",
                "per_turn_stress_damage",
                "reset_hp",
                "use_previous_monster_class_hp",
                "add_current_hp",
            ],
        )?;
        let empty = text(r, "captor_empty_monster_class")?.to_string();
        g.monster_document(&empty)?;
        s.full = Some(CaptorFull {
            empty,
            release_on_death: flag(r, "release_on_death", true)?,
            release_on_door: flag(r, "release_on_prisoner_at_deaths_door", false)?,
            release_on_affliction: flag(r, "release_on_prisoner_affliction", false)?,
            damage: r
                .param("per_turn_damage_percent")
                .map(|_| number(r, "per_turn_damage_percent"))
                .transpose()?
                .unwrap_or(0.),
            stress: r
                .param("per_turn_stress_damage")
                .map(|_| integer(r, "per_turn_stress_damage"))
                .transpose()?
                .unwrap_or(0),
            switch_on_death: flag(r, "switch_class_on_death", true)?,
            reset_hp: flag(r, "reset_hp", true)?,
            previous_hp: flag(r, "use_previous_monster_class_hp", false)?,
            add_hp: flag(r, "add_current_hp", false)?,
            effects: effects(g, r, "release_effects")?,
        });
    }
    if let Some(r) = d.records_keyed("controller").next() {
        known(r, &["stress_per_controlled_turn"])?;
        s.controller_stress = Some(integer(r, "stress_per_controlled_turn")?);
    }
    if let Some(r) = d.records_keyed("companion").next() {
        known(r, &["monster_class", "buffs", "heal_per_turn_percent"])?;
        s.companion = Some(Companion {
            family: text(r, "monster_class")?.into(),
            buffs: words(r, "buffs")
                .iter()
                .map(|id| read_buff(&g.buffs, id))
                .collect::<Result<Vec<_>>>()?,
            heal: number(r, "heal_per_turn_percent")?,
        });
    }
    if let Some(r) = d.records_keyed("skill_reaction").next() {
        known(
            r,
            &[
                "was_hit_performer_effects",
                "was_killed_other_monsters_effects",
                "was_killed_by_hero_effects",
            ],
        )?;
        s.reactions = Reactions {
            hit_performer: effects(g, r, "was_hit_performer_effects")?,
            killed_others: effects(g, r, "was_killed_other_monsters_effects")?,
            killed_by_hero: effects(g, r, "was_killed_by_hero_effects")?,
        };
    }
    if let Some(r) = d.records_keyed("death_damage").next() {
        known(r, &["target_base_class_id", "target_damage"])?;
        s.death_damage = Some((
            text(r, "target_base_class_id")?.into(),
            integer(r, "target_damage")?,
        ));
    }
    if let Some(r) = d.records_keyed("torchlight_modifier").next() {
        known(r, &["min", "max"])?;
        s.torch = Some([
            u8::try_from(integer(r, "min")?).map_err(|_| "invalid torch min")?,
            u8::try_from(integer(r, "max")?).map_err(|_| "invalid torch max")?,
        ]);
    }
    if let Some(r) = d.records_keyed("battle_backdrop").next() {
        known(r, &["background_name", "animation", "isFlat"])?;
        s.backdrop = Some((
            text(r, "background_name")?.into(),
            text(r, "animation")?.into(),
            flag(r, "isFlat", false)?,
        ));
    }
    if let Some(r) = d.records_keyed("audio_modifier").next() {
        known(r, &["ambience_parameter_ids", "ambience_parameter_values"])?;
        let ids = words(r, "ambience_parameter_ids");
        let vs = words(r, "ambience_parameter_values");
        if ids.len() != vs.len() {
            return Err("audio parameter mismatch".into());
        }
        s.audio = ids
            .into_iter()
            .zip(vs.iter().map(|v| scalar(v)).collect::<Result<Vec<_>>>()?)
            .collect();
    }
    if let Some(r) = d.records_keyed("tutorial").next() {
        known(r, &["id"])?;
        s.tutorial = Some(text(r, "id")?.into());
    }
    if let Some(r) = d.records_keyed("mash_modifier").next() {
        known(
            r,
            &["disable_additional_mash_for_infestation_sequence_on_death"],
        )?;
        s.end_infestation = flag(
            r,
            "disable_additional_mash_for_infestation_sequence_on_death",
            false,
        )?;
    }
    if let Some(r) = d.records_keyed("death_class").next() {
        s.death.transform_instead = !flag(r, "can_die_from_damage", true)?;
        s.death.carry_min = r
            .param("carry_over_hp_min_percent")
            .map(|_| number(r, "carry_over_hp_min_percent"))
            .transpose()?;
        if s.death.carry_min.is_some_and(|n| !(0. ..=1.).contains(&n)) {
            return Err("invalid death carry HP minimum".into());
        }
        s.death.previous_hp = flag(r, "use_previous_monster_hp", false)?;
        s.death.change_effects = effects(g, r, "change_class_effects")?;
    }
    Ok(())
}

pub(super) fn skill(r: &Record) -> Result<crate::special::BossSkill> {
    let mut rank_damage = [0.; 4];
    if let Some(p) = r.param("rank_damage_modifiers") {
        if p.values.len() != 4 {
            return Err("rank damage modifiers require four values".into());
        }
        for (i, v) in p.values.iter().enumerate() {
            rank_damage[i] = scalar(&v.text)?;
        }
    }
    let s = crate::special::BossSkill {
        can_miss: flag(r, "can_miss", true)?,
        knowledgeable: flag(r, "is_knowledgeable", true)?,
        user_selected: flag(r, "is_user_selected_targets", false)?,
        starting_cooldown: r
            .param("starting_cooldown")
            .map(|_| {
                integer(r, "starting_cooldown")
                    .and_then(|n| u32::try_from(n).map_err(|_| "invalid starting cooldown".into()))
            })
            .transpose()?
            .unwrap_or(0),
        rank_damage,
        damage_heal: words(r, "damage_heal_base_class_ids"),
        teleport: text(r, "type").ok() == Some("teleport"),
        presentation_rerank: flag(
            r,
            "is_monster_rerank_valid_on_friendly_presentation_end",
            true,
        )?,
    };
    s.validate()?;
    Ok(s)
}
