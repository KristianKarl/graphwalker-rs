//! GraphWalker testing itself.
//!
//! This is a black-box integration test (only `graphwalker-core`'s public
//! API is used) that dogfoods the engine: a *meta-model* is built where each
//! vertex/edge represents a specific `graphwalker-core` capability rather
//! than an application state, and that meta-model is then walked with the
//! crate's own [`Machine`], [`PathGenerator`] and [`StopCondition`]. This
//! way the traversal order itself is a live product of the code under test,
//! and every element visited during that traversal triggers a real
//! assertion against the engine.
//!
//! # Two testing styles used here
//!
//! Most scenarios are wired directly onto the outer meta-model's own edges
//! and vertices (e.g. guards, local/global action scripts, requirements):
//! the real single outer walk exercises them, and [`dispatch`] asserts on
//! the outer [`Machine`]'s state right after the element is traversed.
//!
//! Some capabilities can't be observed mid-walk without derailing the outer
//! coverage-driven traversal (e.g. a guard that must always evaluate to
//! `false`, or a stop condition/generator that needs its own independent
//! walk to completion). For those, the dispatched function builds a small
//! throwaway [`RuntimeModel`] + [`ExecutionContext`]/[`Machine`] and asserts
//! against that nested run instead. Each scenario function documents which
//! style it uses and why.
//!
//! # Meta-model shape
//!
//! The meta-model is a single cycle: `v_Start` through eleven `e_*` edges,
//! each named after the capability it exercises, looping back via
//! `e_restart`. There are no branch points, so with a fixed `SEED` the outer
//! walk is fully deterministic and `EdgeCoverage(100)` is reached in exactly
//! one lap. See [`build_meta_model`] for the full vertex/edge list and
//! [`dispatch`] for the name -> assertion mapping.
//!
//! # Scope
//!
//! Covered: `Random` and `WeightedRandom` generators; the `Combined`
//! (concatenated) generator; `EdgeCoverage`, `ReachedVertex` and `Combined`
//! stop conditions; guards; local/global action scripts; shared-state
//! portals; multi-model `Machine` instances; requirement tracking.
//!
//! Not yet covered (candidates for future scenarios): `QuickRandom`,
//! `AStar`, `ShortestAllPaths`, `Predefined` and `NewYorkStreetSweeper`
//! generators; `VertexCoverage`, `RequirementCoverage`,
//! `DependencyEdgeCoverage`, `ReachedEdge`, `ReachedSharedState`,
//! `TimeDuration`, `Never`, `Alternative`, `PredefinedPath` and
//! `InternalState` stop conditions.
//!
//! # Extending
//!
//! To add a scenario: add a vertex/edge pair to [`build_meta_model`] (or
//! attach a guard/action/requirement to an existing edge if the outer walk
//! can exercise it directly), add its name to the `match` in [`dispatch`],
//! and write an `assert_*` function following the pattern of the existing
//! ones — panic messages should be prefixed with `self-walk: <element name>`
//! so failures are attributable at a glance.

use graphwalker_core::condition::StopCondition;
use graphwalker_core::generator::PathGenerator;
use graphwalker_core::machine::{ExecutionContext, Machine};
use graphwalker_core::model::{
    Action, EdgeBuilder, EdgeIndex, ElementIndex, Guard, ModelBuilder, Requirement,
    RequirementStatus, RuntimeModel, VertexBuilder, VertexIndex,
};

/// Fixed seed for every walk in this file, so a failure is reproducible.
const SEED: u64 = 1234;
/// Safety cap on loop iterations: a regression that breaks termination in
/// the generator/stop-condition machinery should fail the test, not hang it.
const MAX_STEPS: usize = 100;

/// Builds the outer meta-model: a single deterministic cycle through eleven
/// `e_*` edges, one per capability under test, looping back via `e_restart`.
/// See the module docs for the full rationale.
fn build_meta_model() -> RuntimeModel {
    let v_start = VertexBuilder::new().id("v_start").name("v_Start");
    let v_vertex_visited = VertexBuilder::new()
        .id("v_vertex_visited")
        .name("v_VertexVisited")
        .add_action(Action::new("flag_guard=true"));
    let v_guard_passed = VertexBuilder::new()
        .id("v_guard_passed")
        .name("v_GuardPassed");
    let v_guard_blocked = VertexBuilder::new()
        .id("v_guard_blocked")
        .name("v_GuardBlocked");
    let v_action_executed = VertexBuilder::new()
        .id("v_action_executed")
        .name("v_ActionExecuted");
    let v_global_action_executed = VertexBuilder::new()
        .id("v_global_action_executed")
        .name("v_GlobalActionExecuted");
    let v_shared_state_joined = VertexBuilder::new()
        .id("v_shared_state_joined")
        .name("v_SharedStateJoined");
    let v_coverage_verified = VertexBuilder::new()
        .id("v_coverage_verified")
        .name("v_CoverageVerified");
    let v_reached_verified = VertexBuilder::new()
        .id("v_reached_verified")
        .name("v_ReachedVerified");
    let v_requirement_verified = VertexBuilder::new()
        .id("v_requirement_verified")
        .name("v_RequirementVerified");
    let v_weighted_verified = VertexBuilder::new()
        .id("v_weighted_verified")
        .name("v_WeightedVerified");
    let v_combined_stop_verified = VertexBuilder::new()
        .id("v_combined_stop_verified")
        .name("v_CombinedStopVerified");
    let v_concatenated_generators_verified = VertexBuilder::new()
        .id("v_concatenated_generators_verified")
        .name("v_ConcatenatedGeneratorsVerified");
    let v_multi_model_verified = VertexBuilder::new()
        .id("v_multi_model_verified")
        .name("v_MultiModelVerified");

    let mut mb = ModelBuilder::new();

    mb.add_edge(
        EdgeBuilder::new()
            .id("e_visit_vertex")
            .name("e_visitVertex")
            .source_vertex(v_start.clone())
            .target_vertex(v_vertex_visited.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_guard_true")
            .name("e_guardTrue")
            .source_vertex(v_vertex_visited.clone())
            .target_vertex(v_guard_passed.clone())
            .guard(Guard::new("flag_guard == true")),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_guard_false")
            .name("e_guardFalse")
            .source_vertex(v_guard_passed.clone())
            .target_vertex(v_guard_blocked.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_local_action")
            .name("e_localAction")
            .source_vertex(v_guard_blocked.clone())
            .target_vertex(v_action_executed.clone())
            .actions(vec![
                Action::new("counter=1"),
                Action::new("counter++"),
                Action::new("counter++"),
            ]),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_global_action")
            .name("e_globalAction")
            .source_vertex(v_action_executed.clone())
            .target_vertex(v_global_action_executed.clone())
            .actions(vec![
                Action::new("global.gcounter=10"),
                Action::new("global.gcounter++"),
            ]),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_shared_state")
            .name("e_sharedState")
            .source_vertex(v_global_action_executed.clone())
            .target_vertex(v_shared_state_joined.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_stop_condition_coverage")
            .name("e_stopConditionCoverage")
            .source_vertex(v_shared_state_joined.clone())
            .target_vertex(v_coverage_verified.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_stop_condition_reached")
            .name("e_stopConditionReached")
            .source_vertex(v_coverage_verified.clone())
            .target_vertex(v_reached_verified.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_requirement")
            .name("e_requirement")
            .source_vertex(v_reached_verified.clone())
            .target_vertex(v_requirement_verified.clone())
            .add_requirement(Requirement::new("REQ_selfwalk")),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_weighted_random")
            .name("e_weightedRandom")
            .source_vertex(v_requirement_verified.clone())
            .target_vertex(v_weighted_verified.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_combined_stop_condition")
            .name("e_combinedStopCondition")
            .source_vertex(v_weighted_verified.clone())
            .target_vertex(v_combined_stop_verified.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_concatenated_generators")
            .name("e_concatenatedGenerators")
            .source_vertex(v_combined_stop_verified.clone())
            .target_vertex(v_concatenated_generators_verified.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_multi_model")
            .name("e_multiModel")
            .source_vertex(v_concatenated_generators_verified.clone())
            .target_vertex(v_multi_model_verified.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_restart")
            .name("e_restart")
            .source_vertex(v_multi_model_verified.clone())
            .target_vertex(v_start.clone()),
    );

    mb.build()
}

/// Drives the outer meta-model to completion with `graphwalker-core`'s own
/// `Random` generator under an `EdgeCoverage(100)` stop condition, running
/// [`dispatch`] on every element visited along the way.
#[test]
fn core_self_walk() {
    let model = build_meta_model();
    let mut ctx = ExecutionContext::new_with_seed(model, SEED);
    ctx.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let gen = PathGenerator::random(StopCondition::EdgeCoverage(100));
    let mut machine = Machine::new_with_seed(vec![(ctx, gen)], SEED).unwrap();

    let mut steps = 0;
    while machine.has_next_step() {
        machine.get_next_step().unwrap();
        steps += 1;
        assert!(
            steps < MAX_STEPS,
            "self-walk did not terminate within {MAX_STEPS} steps"
        );

        let element = machine.current_context().current_element().unwrap();
        let name = element_name(&machine, element);
        dispatch(&machine, &name);
    }

    assert!(
        machine.get_fulfilment(0) >= 0.999999,
        "self-walk finished without full edge coverage"
    );
}

/// Resolves a traversed [`ElementIndex`] back to the vertex/edge `name()`
/// it was built with in [`build_meta_model`], used as the dispatch key.
fn element_name(machine: &Machine, element: ElementIndex) -> String {
    let model = machine.current_context().model();
    match element {
        ElementIndex::Vertex(vi) => model.vertex(vi).name().unwrap_or_default().to_string(),
        ElementIndex::Edge(ei) => model.edge(ei).name().unwrap_or_default().to_string(),
    }
}

/// Maps a meta-model element name to the scenario that verifies it. Unnamed
/// entries (`_`) are pass-through vertices/edges with no dedicated check.
fn dispatch(machine: &Machine, name: &str) {
    match name {
        "v_VertexVisited" => assert_vertex_visited(machine),
        "e_guardTrue" => assert_guard_true(machine),
        "e_guardFalse" => assert_guard_false_blocks_edge(),
        "e_localAction" => assert_local_action(machine),
        "e_globalAction" => assert_global_action(machine),
        "e_sharedState" => assert_shared_state_join(),
        "e_stopConditionCoverage" => assert_stop_condition_coverage(),
        "e_stopConditionReached" => assert_stop_condition_reached(),
        "e_requirement" => assert_requirement(machine),
        "e_weightedRandom" => assert_weighted_random(),
        "e_combinedStopCondition" => assert_combined_stop_condition(),
        "e_concatenatedGenerators" => assert_concatenated_generators(),
        "e_multiModel" => assert_multi_model(),
        _ => {}
    }
}

// -- v_VertexVisited: visiting a vertex updates the context's own bookkeeping --

fn assert_vertex_visited(machine: &Machine) {
    let ctx = machine.current_context();
    let vi = VertexIndex(1); // v_vertex_visited
    assert!(
        ctx.is_at_vertex(),
        "self-walk: v_VertexVisited — context must be at a vertex"
    );
    assert!(
        ctx.is_visited(ElementIndex::Vertex(vi)),
        "self-walk: v_VertexVisited — visiting a vertex must mark it visited"
    );
    assert_eq!(
        ctx.visit_count(ElementIndex::Vertex(vi)),
        1,
        "self-walk: v_VertexVisited — visit count must be exactly 1 on first visit"
    );
}

// -- e_guardTrue: a satisfied guard lets the real walk cross the edge --

fn assert_guard_true(machine: &Machine) {
    let val = machine
        .current_context()
        .get_attribute("flag_guard")
        .expect("self-walk: e_guardTrue — `flag_guard` must be set by the vertex action");
    assert!(
        val.as_bool().unwrap_or(false),
        "self-walk: e_guardTrue — guard `flag_guard == true` must have been satisfied to get here"
    );
}

// -- e_guardFalse: a guard evaluating to false blocks an edge from being available --

fn assert_guard_false_blocks_edge() {
    let va = VertexBuilder::new().id("va").name("A");
    let vb = VertexBuilder::new().id("vb").name("B");
    let mut mb = ModelBuilder::new();
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_blocked")
            .name("e_blocked")
            .source_vertex(va.clone())
            .target_vertex(vb.clone())
            .guard(Guard::new("false")),
    );
    let model = mb.build();
    let ctx = ExecutionContext::new(model);

    assert!(
        !ctx.is_edge_available(EdgeIndex(0)),
        "self-walk: e_guardFalse — an edge guarded by `false` must not be available"
    );
    let candidates = [ElementIndex::Edge(EdgeIndex(0))];
    assert!(
        ctx.filter_elements(&candidates).is_empty(),
        "self-walk: e_guardFalse — filter_elements must exclude a guard-blocked edge"
    );
}

// -- e_localAction: local-scope action scripts mutate context-local state --

fn assert_local_action(machine: &Machine) {
    let val = machine
        .current_context()
        .get_attribute("counter")
        .expect("self-walk: e_localAction — `counter` must be set by the edge action");
    assert_eq!(
        val.as_int().unwrap(),
        3,
        "self-walk: e_localAction — edge action script did not execute as expected"
    );
}

// -- e_globalAction: `global.` scoped action scripts write to the shared scope --

fn assert_global_action(machine: &Machine) {
    let val = machine
        .current_context()
        .get_attribute("global.gcounter")
        .expect("self-walk: e_globalAction — `global.gcounter` must be set by the edge action");
    assert_eq!(
        val.as_int().unwrap(),
        11,
        "self-walk: e_globalAction — global-scoped edge action script did not execute as expected"
    );
}

// -- e_sharedState: two contexts rendezvous on a shared-state vertex --

fn assert_shared_state_join() {
    let v_shared = VertexBuilder::new()
        .id("v_shared")
        .name("Shared")
        .shared_state("SELF_WALK_SHARED");
    let v_a = VertexBuilder::new().id("v_a").name("A");
    let v_b = VertexBuilder::new().id("v_b").name("B");

    let mut mb1 = ModelBuilder::new();
    mb1.add_edge(
        EdgeBuilder::new()
            .id("e1")
            .name("e1")
            .source_vertex(v_a.clone())
            .target_vertex(v_shared.clone()),
    );
    let model1 = mb1.build();

    let mut mb2 = ModelBuilder::new();
    mb2.add_edge(
        EdgeBuilder::new()
            .id("e2")
            .name("e2")
            .source_vertex(v_shared.clone())
            .target_vertex(v_b.clone()),
    );
    let model2 = mb2.build();

    let mut ctx1 = ExecutionContext::new_with_seed(model1, SEED);
    ctx1.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let gen1 = PathGenerator::random(StopCondition::EdgeCoverage(100));

    let mut ctx2 = ExecutionContext::new_with_seed(model2, SEED);
    ctx2.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let gen2 = PathGenerator::random(StopCondition::EdgeCoverage(100));

    let mut machine = Machine::new_with_seed(vec![(ctx1, gen1), (ctx2, gen2)], SEED).unwrap();

    let mut visited_contexts = std::collections::HashSet::new();
    let mut steps = 0;
    while machine.has_next_step() && steps < MAX_STEPS {
        machine.get_next_step().unwrap();
        visited_contexts.insert(machine.current_context_index());
        steps += 1;
    }

    assert_eq!(
        visited_contexts.len(),
        2,
        "self-walk: e_sharedState — both contexts must be visited via the shared-state portal"
    );
    assert!(
        machine
            .context(0)
            .is_visited(ElementIndex::Edge(EdgeIndex(0))),
        "self-walk: e_sharedState — model 1's edge must be visited"
    );
    assert!(
        machine
            .context(1)
            .is_visited(ElementIndex::Edge(EdgeIndex(0))),
        "self-walk: e_sharedState — model 2's edge must be visited"
    );
}

// -- e_stopConditionCoverage: EdgeCoverage(100) only fulfils once every edge is visited --

fn assert_stop_condition_coverage() {
    let v_a = VertexBuilder::new().id("va").name("A");
    let v_b = VertexBuilder::new().id("vb").name("B");
    let mut mb = ModelBuilder::new();
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_ab")
            .name("e_AB")
            .source_vertex(v_a.clone())
            .target_vertex(v_b.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_ba")
            .name("e_BA")
            .source_vertex(v_b.clone())
            .target_vertex(v_a.clone()),
    );
    let model = mb.build();

    let mut ctx = ExecutionContext::new_with_seed(model, SEED);
    ctx.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let gen = PathGenerator::random(StopCondition::EdgeCoverage(100));
    let mut machine = Machine::new_with_seed(vec![(ctx, gen)], SEED).unwrap();

    assert!(
        machine.get_fulfilment(0) < 0.999999,
        "self-walk: e_stopConditionCoverage — must not be fulfilled before any edge is visited"
    );

    let mut steps = 0;
    while machine.has_next_step() && steps < MAX_STEPS {
        machine.get_next_step().unwrap();
        steps += 1;
    }

    assert!(
        machine.get_fulfilment(0) >= 0.999999,
        "self-walk: e_stopConditionCoverage — must be fulfilled once every edge is visited"
    );
    assert!(
        !machine.has_next_step(),
        "self-walk: e_stopConditionCoverage — a fulfilled stop condition must end the walk"
    );
}

// -- e_stopConditionReached: ReachedVertex latches fulfilled once its target is hit --

fn assert_stop_condition_reached() {
    let v_a = VertexBuilder::new().id("va").name("A");
    let v_target = VertexBuilder::new().id("v_target").name("Target");
    let mut mb = ModelBuilder::new();
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_at")
            .name("e_AT")
            .source_vertex(v_a.clone())
            .target_vertex(v_target.clone()),
    );
    let model = mb.build();

    let mut ctx = ExecutionContext::new_with_seed(model, SEED);
    ctx.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let stop = StopCondition::reached_vertex("Target");

    assert!(
        !stop.is_fulfilled(&ctx),
        "self-walk: e_stopConditionReached — must not be fulfilled before the target is reached"
    );

    let gen = PathGenerator::random(stop.clone());
    let mut machine = Machine::new_with_seed(vec![(ctx, gen)], SEED).unwrap();

    let mut steps = 0;
    while machine.has_next_step() && steps < MAX_STEPS {
        machine.get_next_step().unwrap();
        steps += 1;
    }

    assert!(
        stop.is_fulfilled(machine.current_context()),
        "self-walk: e_stopConditionReached — must be fulfilled after reaching the target vertex"
    );
}

// -- e_requirement: visiting an element with a requirement marks it Passed --

fn assert_requirement(machine: &Machine) {
    assert_eq!(
        machine
            .context(0)
            .requirements_with_status(RequirementStatus::Passed),
        1,
        "self-walk: e_requirement — requirement attached to the edge must be marked Passed"
    );
}

// -- e_weightedRandom: WeightedRandom favors higher-weight edges over many samples --

fn assert_weighted_random() {
    let v_decision = VertexBuilder::new().id("v_decision").name("Decision");
    let v_next = VertexBuilder::new().id("v_next").name("Next");
    let mut mb = ModelBuilder::new();
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_hi")
            .name("e_hi")
            .source_vertex(v_decision.clone())
            .target_vertex(v_next.clone())
            .weight(0.9),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_lo")
            .name("e_lo")
            .source_vertex(v_decision.clone())
            .target_vertex(v_next.clone())
            .weight(0.1),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_back")
            .name("e_back")
            .source_vertex(v_next.clone())
            .target_vertex(v_decision.clone()),
    );
    let model = mb.build();

    let mut ctx = ExecutionContext::new_with_seed(model, SEED);
    ctx.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let gen = PathGenerator::weighted_random(StopCondition::Length(400));
    let mut machine = Machine::new_with_seed(vec![(ctx, gen)], SEED).unwrap();

    while machine.has_next_step() {
        machine.get_next_step().unwrap();
    }

    let ctx = machine.current_context();
    let hi_visits = ctx.visit_count(ElementIndex::Edge(EdgeIndex(0)));
    let lo_visits = ctx.visit_count(ElementIndex::Edge(EdgeIndex(1)));
    assert!(
        hi_visits > lo_visits * 3,
        "self-walk: e_weightedRandom — a 0.9 vs 0.1 weighted edge should be picked far more often \
         (hi={hi_visits}, lo={lo_visits})"
    );
}

// -- e_combinedStopCondition: `StopCondition::Combined` is an AND over its sub-conditions --

fn assert_combined_stop_condition() {
    let v_a = VertexBuilder::new().id("va").name("A");
    let v_b = VertexBuilder::new().id("vb").name("B");
    let mut mb = ModelBuilder::new();
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_ab")
            .name("e_AB")
            .source_vertex(v_a.clone())
            .target_vertex(v_b.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_ba")
            .name("e_BA")
            .source_vertex(v_b.clone())
            .target_vertex(v_a.clone()),
    );
    let model = mb.build();

    let stop = StopCondition::Combined(vec![
        StopCondition::EdgeCoverage(100),
        StopCondition::Length(4),
    ]);

    let mut ctx = ExecutionContext::new_with_seed(model, SEED);
    ctx.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let gen = PathGenerator::random(stop.clone());
    let mut machine = Machine::new_with_seed(vec![(ctx, gen)], SEED).unwrap();

    machine.get_next_step().unwrap(); // land on the start vertex
    assert!(
        !stop.is_fulfilled(machine.current_context()),
        "self-walk: e_combinedStopCondition — must not be fulfilled with only the start vertex visited"
    );

    let mut steps = 1;
    while machine.has_next_step() && steps < MAX_STEPS {
        machine.get_next_step().unwrap();
        steps += 1;
    }

    let ctx = machine.current_context();
    assert!(
        StopCondition::EdgeCoverage(100).is_fulfilled(ctx),
        "self-walk: e_combinedStopCondition — the EdgeCoverage sub-condition must be individually satisfied"
    );
    assert!(
        StopCondition::Length(4).is_fulfilled(ctx),
        "self-walk: e_combinedStopCondition — the Length sub-condition must be individually satisfied"
    );
    assert!(
        stop.is_fulfilled(ctx),
        "self-walk: e_combinedStopCondition — Combined (AND) must be fulfilled once every sub-condition is"
    );
}

// -- e_concatenatedGenerators: `PathGenerator::combined` runs generators in sequence --

fn assert_concatenated_generators() {
    let v_a = VertexBuilder::new().id("va").name("A");
    let v_b = VertexBuilder::new().id("vb").name("B");
    let v_c = VertexBuilder::new().id("vc").name("C");
    let v_d = VertexBuilder::new().id("vd").name("D");
    let mut mb = ModelBuilder::new();
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_ab")
            .name("e_AB")
            .source_vertex(v_a.clone())
            .target_vertex(v_b.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_bc")
            .name("e_BC")
            .source_vertex(v_b.clone())
            .target_vertex(v_c.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_cd")
            .name("e_CD")
            .source_vertex(v_c.clone())
            .target_vertex(v_d.clone()),
    );
    mb.add_edge(
        EdgeBuilder::new()
            .id("e_da")
            .name("e_DA")
            .source_vertex(v_d.clone())
            .target_vertex(v_a.clone()),
    );
    let model = mb.build();

    // The first generator alone would stop after ~1 edge (Length(2) counts the
    // start vertex too); only the second, concatenated generator can finish
    // covering all 4 edges.
    let combined = PathGenerator::combined(vec![
        PathGenerator::random(StopCondition::Length(2)),
        PathGenerator::random(StopCondition::EdgeCoverage(100)),
    ]);

    let mut ctx = ExecutionContext::new_with_seed(model, SEED);
    ctx.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let mut machine = Machine::new_with_seed(vec![(ctx, combined)], SEED).unwrap();

    let mut steps = 0;
    while machine.has_next_step() && steps < MAX_STEPS {
        machine.get_next_step().unwrap();
        steps += 1;
    }

    assert!(
        steps > 2,
        "self-walk: e_concatenatedGenerators — the walk must outlive the first generator's own stop condition"
    );
    assert!(
        StopCondition::EdgeCoverage(100).is_fulfilled(machine.current_context()),
        "self-walk: e_concatenatedGenerators — the second generator must take over and finish covering all edges \
         once the first generator's stop condition is satisfied"
    );
}

// -- e_multiModel: a single `Machine` walks several independent (non-shared-state) models --

fn assert_multi_model() {
    let v_x1 = VertexBuilder::new().id("vx1").name("X1");
    let v_x2 = VertexBuilder::new().id("vx2").name("X2");
    let mut mbx = ModelBuilder::new();
    mbx.add_edge(
        EdgeBuilder::new()
            .id("e_x1_x2")
            .name("e_X1X2")
            .source_vertex(v_x1.clone())
            .target_vertex(v_x2.clone()),
    );
    let model_x = mbx.build();

    let v_y1 = VertexBuilder::new().id("vy1").name("Y1");
    let v_y2 = VertexBuilder::new().id("vy2").name("Y2");
    let v_y3 = VertexBuilder::new().id("vy3").name("Y3");
    let mut mby = ModelBuilder::new();
    mby.add_edge(
        EdgeBuilder::new()
            .id("e_y1_y2")
            .name("e_Y1Y2")
            .source_vertex(v_y1.clone())
            .target_vertex(v_y2.clone()),
    );
    mby.add_edge(
        EdgeBuilder::new()
            .id("e_y2_y3")
            .name("e_Y2Y3")
            .source_vertex(v_y2.clone())
            .target_vertex(v_y3.clone()),
    );
    let model_y = mby.build();

    let mut ctx_x = ExecutionContext::new_with_seed(model_x, SEED);
    ctx_x.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let gen_x = PathGenerator::random(StopCondition::EdgeCoverage(100));

    let mut ctx_y = ExecutionContext::new_with_seed(model_y, SEED);
    ctx_y.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let gen_y = PathGenerator::random(StopCondition::EdgeCoverage(100));

    let mut machine = Machine::new_with_seed(vec![(ctx_x, gen_x), (ctx_y, gen_y)], SEED).unwrap();
    assert_eq!(
        machine.context_count(),
        2,
        "self-walk: e_multiModel — Machine must track both registered models"
    );

    let mut steps = 0;
    while machine.has_next_step() && steps < MAX_STEPS {
        machine.get_next_step().unwrap();
        steps += 1;
    }

    // No shared state links the two models, so a model's context must run to
    // completion before the other one is ever selected: the recorded context
    // indices must not interleave.
    let context_indices: Vec<usize> = machine.execution_path().iter().map(|(i, _)| *i).collect();
    let switch_points = context_indices.windows(2).filter(|w| w[0] != w[1]).count();
    assert_eq!(
        switch_points, 1,
        "self-walk: e_multiModel — independent models must not interleave, found {switch_points} context switches in {context_indices:?}"
    );

    assert!(
        StopCondition::EdgeCoverage(100).is_fulfilled(machine.context(0)),
        "self-walk: e_multiModel — model X must reach full edge coverage"
    );
    assert!(
        StopCondition::EdgeCoverage(100).is_fulfilled(machine.context(1)),
        "self-walk: e_multiModel — model Y must reach full edge coverage"
    );
}
