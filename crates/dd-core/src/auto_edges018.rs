use super::*;
#[test]
#[ignore = "installed full source; actual DD escape sacrifices, solo boundary, strict reuse, XP ownership and restore"]
fn installed_dd_retreat_strict_capacity_and_failure_xp_edges() {
    for solo in [false, true] {
        let mut s = controlled(1);
        if solo {
            s.state.city.party.truncate(1);
        }
        let before = s.observe().unwrap();
        let ids = before.town.party.clone();
        let q = before
            .board
            .unwrap()
            .quests
            .into_iter()
            .find(|q| q.plot.as_deref() == Some("plot_darkest_dungeon_1"))
            .unwrap();
        let source = s
            .state
            .board
            .entries
            .iter()
            .find(|e| e.id == q.id)
            .unwrap()
            .clone();
        let mut marked = s.clone();
        let id = ids[0];
        marked
            .state
            .city
            .campaign
            .as_mut()
            .unwrap()
            .automation
            .as_mut()
            .unwrap()
            .marked
            .insert(id);
        assert_eq!(
            marked.observe().unwrap().town.roster_capacity,
            before.town.roster_capacity + 1
        );
        let cp = marked.checkpoint().unwrap();
        assert!(
            marked
                .preparation_eligibility(&source)
                .unwrap_err()
                .contains("never again")
        );
        assert_eq!(cp, marked.checkpoint().unwrap());
        prepare(&mut s, q.id);
        check(&mut s, |s| {
            s.expedition_action(crate::opening015::Action::Leave { complete: false })
        });
        let after = s.observe().unwrap();
        assert_eq!(after.week, before.week + 1);
        assert_eq!(after.total_deaths - before.total_deaths, usize::from(!solo));
        assert_eq!(after.town.party.len(), ids.len() - usize::from(!solo));
        assert!(
            !after
                .automation
                .as_ref()
                .unwrap()
                .completed
                .contains(&"plot_darkest_dungeon_1".into())
        );
        assert!(
            after
                .town
                .heroes
                .iter()
                .filter(|h| h.alive && ids.contains(&h.guid))
                .all(|h| after
                    .automation
                    .as_ref()
                    .unwrap()
                    .failure_xp_stacks
                    .get(&h.guid)
                    == Some(&1))
        );
        let t = s.town();
        let h = t.state.party[0];
        assert_eq!(
            t.xp_bonus018(h),
            t.state
                .campaign
                .as_ref()
                .unwrap()
                .automation
                .as_ref()
                .unwrap()
                .rules
                .failure_xp
        );
        eprintln!(
            "018 DD escape solo={solo} deaths={} remaining={} week={} real native return and restore equal",
            after.total_deaths - before.total_deaths,
            after.town.party.len(),
            after.week
        );
    }
}

#[test]
#[ignore = "installed full source; permissive never-again real entry and source minimum stress/Buff"]
fn installed_permissive_reentry_retains_source_buff_stress_and_cold_save() {
    let mut s = controlled_setting(1, "permissive");
    let id = s.state.city.party[0];
    s.state
        .city
        .campaign
        .as_mut()
        .unwrap()
        .automation
        .as_mut()
        .unwrap()
        .marked
        .insert(id);
    let minimum = s.definition.automation.as_ref().unwrap().minimum_stress;
    let buffs = s
        .definition
        .automation
        .as_ref()
        .unwrap()
        .reentry_buffs
        .clone();
    assert!(!buffs.is_empty());
    let q = s
        .observe()
        .unwrap()
        .board
        .unwrap()
        .quests
        .into_iter()
        .find(|q| q.plot.as_deref() == Some("plot_darkest_dungeon_1"))
        .unwrap();
    prepare(&mut s, q.id);
    let o = s.observe().unwrap().expedition.unwrap();
    assert!(o.heroes.iter().find(|h| h.guid == id).unwrap().stress >= minimum);
    let cp = s.checkpoint().unwrap();
    let raw: Value = serde_json::from_str(&cp).unwrap();
    let raid: Value = serde_json::from_str(raw["active"]["snapshot"].as_str().unwrap()).unwrap();
    let hero = raid["heroes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["guid"] == id)
        .unwrap();
    assert_eq!(
        hero["base"]["mechanics"]["hero"]["host_passive_buffs"],
        serde_json::to_value(&buffs).unwrap()
    );
    assert_eq!(
        cp,
        Session::restore(&cp, &s.definition)
            .unwrap()
            .checkpoint()
            .unwrap()
    );
    eprintln!(
        "018 permissive source minimum={minimum}, source reentry Buff count={}, actual embark and restore equal",
        buffs.len()
    );
}
#[test]
#[ignore = "installed class activity source; no test stat boost"]
fn installed_flagellant_class_activities_and_side_effects_obey_original_limits() {
    let s = fixture();
    let mut t = s.town();
    t.state.wallet.insert("gold:".into(), 20000);
    t.state.caretaker.clear();
    t.state.recruit_events.insert("first_flagellant".into());
    let mut h = t.generate("flagellant", 0, 0, 0, 0, 0).unwrap();
    h.quirks.clear();
    h.state.stress = 90;
    let id = h.guid;
    t.install_hero_purchases(&h).unwrap();
    t.state.heroes.push(h);
    let source = t.class_activity018("flagellant").unwrap().clone();
    assert_eq!(source.allowed.len(), 3);
    assert_eq!(source.stress, Some([100, 100]));
    for activity in [
        "tavern.bar",
        "tavern.gambling",
        "tavern.brothel",
        "abbey.meditation",
        "abbey.prayer",
    ] {
        assert!(t.treatment_quote(id, activity, &Treatment::Stress).is_err());
    }
    let visible = t.observe().unwrap();
    assert!(visible.actions.iter().any(|c|matches!(&c.action,crate::town016::Action::Queue{hero,facility,treatment:Treatment::Stress} if *hero==id&&facility=="abbey.flagellation")));
    assert!(!visible.actions.iter().any(|c|matches!(&c.action,crate::town016::Action::Queue{hero,facility,..} if *hero==id&&facility.starts_with("tavern."))));
    let original = t.hero(id).unwrap().quirks.clone();
    let job = crate::town016::Job {
        hero: id,
        facility: "abbey.flagellation".into(),
        treatment: Treatment::Stress,
        paid: Default::default(),
        locked_until: None,
    };
    let forbidden_effect = json!({"side_effects":{"chance":1.0,"results":[{"type":"add_quirk","chance":1.0,"data":[{"chance":1.0,"quirk_library_name":"flagellant"}]}]}});
    assert_eq!(t.side_effect(&job, &forbidden_effect).unwrap(), None);
    assert_eq!(t.hero(id).unwrap().quirks, original);
    t.act(crate::town016::Action::Queue {
        hero: id,
        facility: "abbey.flagellation".into(),
        treatment: Treatment::Stress,
    })
    .unwrap();
    t.settle_jobs().unwrap();
    assert_eq!(t.hero(id).unwrap().state.stress, 0);
    t.validate().unwrap();
    let forbidden = t
        .blueprint(t.hero(id).unwrap(), 1, false)
        .unwrap()
        .mechanics
        .hero
        .incompatible_quirks;
    assert!(
        t.hero(id)
            .unwrap()
            .quirks
            .keys()
            .all(|q| !forbidden.contains(q))
    );
    eprintln!(
        "018 Flagellant: five illegal stress services rejected, source 100-point treatment applied; class-forbidden side effects filtered"
    );
}

#[test]
#[ignore = "installed original source; native DD battle retreat returns to map when original flag is false; actual raid departure applies source sacrifice"]
fn installed_dd_native_retreat_then_raid_exit_follows_original_contract_and_replays() {
    let mut s = controlled(1);
    let before = s.observe().unwrap();
    let q = before
        .board
        .unwrap()
        .quests
        .into_iter()
        .find(|q| q.plot.as_deref() == Some("plot_darkest_dungeon_1"))
        .unwrap();
    prepare(&mut s, q.id);
    for _ in 0..2000 {
        let o = s.observe().unwrap().expedition.unwrap();
        if o.battle.is_some() {
            break;
        }
        let a = crate::auto_policy018::raid_action(&o).unwrap();
        check(&mut s, |s| s.expedition_action(a.clone()));
    }
    let retreat = crate::opening015::Action::Fight(crate::Action::Retreat);
    let mut attempts = 0;
    loop {
        let o = s.observe().unwrap().expedition.unwrap();
        if o.battle.is_none() {
            break;
        }
        assert!(
            o.battle
                .unwrap()
                .actions
                .iter()
                .any(|a| a.action == crate::Action::Retreat)
        );
        check(&mut s, |s| s.expedition_action(retreat.clone()));
        attempts += 1;
        assert!(attempts < 20, "fixed original retreat did not succeed");
    }
    let returned = s.observe().unwrap();
    assert_eq!(returned.week, before.week);
    assert_eq!(returned.total_deaths, before.total_deaths);
    assert_eq!(returned.returns, before.returns);
    check(&mut s, |s| {
        s.expedition_action(crate::opening015::Action::Leave { complete: false })
    });
    let after = s.observe().unwrap();
    assert_eq!(after.week, before.week + 1);
    assert_eq!(after.returns, before.returns + 1);
    assert_eq!(after.total_deaths, before.total_deaths + 1);
    assert_eq!(after.town.party.len(), 3);
    let cp = s.checkpoint().unwrap();
    assert!(s.expedition_action(retreat).is_err());
    assert_eq!(cp, s.checkpoint().unwrap());
    eprintln!(
        "018 native DD battle retreat attempts={attempts}, source false returns to map, actual raid departure and source sacrifice, one clock receipt, every step restore equal"
    );
}

#[test]
#[ignore = "installed original quest requirements and planner; controlled roster/plot history, no stat boost"]
fn installed_visible_court_requirements_and_first_blood_goal_precede_later_bosses() {
    let mut s = controlled(0);
    s.state
        .city
        .completed_ids
        .insert("plot_crimson_court_0".into());
    s.state.board.week = None;
    s.refresh_board().unwrap();
    let o = s.observe().unwrap();
    let baron = o
        .board
        .as_ref()
        .unwrap()
        .quests
        .iter()
        .find(|q| q.plot.as_deref() == Some("plot_crimson_court_1"))
        .unwrap();
    assert!(!baron.estate_requirements.is_empty());
    assert!(
        baron
            .estate_requirements
            .iter()
            .any(|(id, n)| o.town.wallet.get(id).copied().unwrap_or(0) < *n)
    );
    let mut memory = crate::auto_policy018::Memory::default();
    for _ in 0..300 {
        let o = s.observe().unwrap();
        let d = crate::auto_policy018::choose(&o, &mut memory).unwrap();
        if let Some(id) = &memory.quest {
            let q = o
                .board
                .as_ref()
                .unwrap()
                .quests
                .iter()
                .find(|q| q.id == *id)
                .unwrap();
            assert_eq!(q.kind, "farm_bloodcrate");
            eprintln!(
                "018 visible requirements reject unowned invitation independently of party level; first blood goal chosen before later Court bosses and DD"
            );
            return;
        }
        match d.action {
            crate::auto_policy018::Action::Town(a) => s.town_action(a).unwrap(),
            crate::auto_policy018::Action::Event(a) => s.event_action(a).unwrap(),
            other => panic!("unexpected preselection {other:?}"),
        }
    }
    panic!("planner did not select first blood goal");
}
