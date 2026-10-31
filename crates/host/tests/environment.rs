mod common;
use common::*;
use lucent_host::{
    bridge::EventInterests,
    scripting::{Kill, RuntimeError},
    testing,
};
use lucent_protocol::{query::Query, script::ScriptOutcome};
use serde_json::json;

#[test]
fn scripts_use_every_environment_method_and_receive_loaded_quest_targets() {
    let (_host, mut session, runtime) = ready_runtime();
    let links = session.attached_link_count().recv_timeout(TIMEOUT).unwrap();
    runtime.eval("quest.js", r#"
        window.swf["probe.emit"]("onExtensionResponse", JSON.stringify({type:"json",dataObj:{cmd:"getQuests",quests:{"802":{
            QuestID:802,sName:"Quest",Rewards:[{ItemID:9,sName:"Reward Blade"}],
            RequiredItems:[{ItemID:10,sName:"Requirement",bTemp:0},{ItemID:11,sName:"Temporary",bTemp:1},{ItemID:12,sName:"Unknown"}]
        }}}})); null;
    "#).wait::<()>(TIMEOUT).unwrap();
    let script = loose(&[(
        "environment.js",
        r#"
        import { environment as e } from "lucent/api";
        import * as script from "lucent/script";
        function check(actual, expected) {
            if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(JSON.stringify({actual,expected}));
        }
        export default function* () {
            check(yield* e.getState(), {
                automation:{boosts:true,drops:true,quests:true},boosts:[],dropPolicy:{acceptAcMemberOnlyDrops:false,acceptAcNonMemberDrops:false,acceptNonAcMemberOnlyDrops:false,acceptNonAcNonMemberDrops:false,rejectUnregisteredDrops:false},
                itemNames:[],itemNotificationNames:[],questAutoRegister:{requirements:false,rewards:false},questIds:[],questRewards:{}
            });
            check((yield* e.updateQuestAutoRegister({rewards:true})).questAutoRegister,{requirements:false,rewards:true});
            const added = yield* e.addQuest(802.9,9.9);
            check(added.itemNames,["Reward Blade"]);
            check(added.questIds,[802]); check(added.questRewards,{802:9});
            check((yield* e.addQuest(802)).itemNames,["Reward Blade"]);
            check((yield* e.setQuestReward(3.9,12.9)).questRewards,{3:12,802:9});
            check((yield* e.clearQuestReward(3)).questRewards,{802:9});
            check((yield* e.removeQuest(3)).questIds,[802]);
            check((yield* e.updateQuestAutoRegister({requirements:true})).itemNames,["Requirement","Reward Blade"]);
            check((yield* e.clearItems()).itemNames,["Requirement","Reward Blade"]);
            check((yield* e.addQuest(NaN)).questIds,[802]);
            check((yield* e.setQuestReward(3,Infinity)).questIds,[802]);
            check((yield* e.clearQuests()).questIds,[]);
            check((yield* e.removeItem("Requirement")).itemNames,["Reward Blade"]);
            check((yield* e.addItem(" Zorbak ")).itemNames,["Reward Blade","Zorbak"]);
            check((yield* e.addItem("zorbak")).itemNames,["Reward Blade","Zorbak"]);
            check((yield* e.addItem("apple")).itemNames,["apple","Reward Blade","Zorbak"]);
            check((yield* e.setItemNotification(" APPLE ",true)).itemNotificationNames,["apple"]);
            check((yield* e.removeItem("APPLE")).itemNotificationNames,[]);
            check((yield* e.clearItems()).itemNames,[]);
            check((yield* e.updateDropPolicy({acceptAcMemberOnlyDrops:true,rejectUnregisteredDrops:true})).dropPolicy,{acceptAcMemberOnlyDrops:true,acceptAcNonMemberDrops:false,acceptNonAcMemberOnlyDrops:false,acceptNonAcNonMemberDrops:false,rejectUnregisteredDrops:true});
            check((yield* e.updateDropPolicy({acceptAcNonMemberDrops:true})).dropPolicy,{acceptAcMemberOnlyDrops:true,acceptAcNonMemberDrops:true,acceptNonAcMemberOnlyDrops:false,acceptNonAcNonMemberDrops:false,rejectUnregisteredDrops:true});
            check((yield* e.addBoost(" XP Boost ")).boosts,["XP Boost"]);
            check((yield* e.removeBoost("xp boost")).boosts,[]);
            yield* e.addBoost("Gold Boost");
            check((yield* e.clearBoosts()).boosts,[]);
            check((yield* e.setAutomationEnabled("drops",false)).automation,{boosts:true,drops:false,quests:true});
            yield* e.addItem("Keep no registrations"); yield* e.addQuest(3,12); yield* e.addBoost("XP Boost");
            const clear = yield* e.clearRegistrations();
            check(clear,{automation:{boosts:true,drops:false,quests:true},boosts:[],dropPolicy:{acceptAcMemberOnlyDrops:true,acceptAcNonMemberDrops:true,acceptNonAcMemberOnlyDrops:false,acceptNonAcNonMemberDrops:false,rejectUnregisteredDrops:true},itemNames:[],itemNotificationNames:[],questAutoRegister:{requirements:true,rewards:true},questIds:[],questRewards:{}});
            yield* script.stop("all environment methods passed");
        }
    "#,
    )]);
    assert_eq!(
        start(&mut session, script, Default::default())
            .wait(TIMEOUT)
            .unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some("all environment methods passed".into())
        }
    );
    assert_eq!(
        session.attached_link_count().recv_timeout(TIMEOUT).unwrap(),
        links
    );
}

#[test]
fn environment_survives_a_killed_script_and_player_restart() {
    let (_host, mut session, _runtime) = ready_runtime();
    let observer = session.attach(EventInterests::ALL).unwrap();
    let run = start(
        &mut session,
        loose(&[(
            "environment.js",
            r#"
        import { environment } from "lucent/api";
        import { packet } from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            yield* environment.addItem("Saved Blade");
            yield* packet.sendToClient("stored");
            yield* script.sleep("1 hour");
        }
    "#,
        )]),
        Default::default(),
    );
    await_event(&observer, "onDebug", Some(&run));
    run.kill();
    assert!(matches!(
        run.wait(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Requested))
    ));
    let expected = json!({"automation":{"boosts":true,"drops":true,"quests":true},"boosts":[],"itemNames":["Saved Blade"],"itemNotificationNames":[],"itemRules":{"buckets":[],"rejectElse":false},"questAutoRegister":{"requirements":false,"rewards":false},"questIds":[],"questRewards":{}});
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&observer.query(Query::Environment {}).unwrap())
            .unwrap(),
        expected
    );
    session.restart(testing::headless_surface()).unwrap();
    let reader = session.attach(EventInterests::ALL).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&reader.query(Query::Environment {}).unwrap())
            .unwrap(),
        expected
    );
}
