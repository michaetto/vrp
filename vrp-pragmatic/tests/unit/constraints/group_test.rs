use super::*;
use crate::extensions::create_typed_actor_groups;
use crate::helpers::*;
use std::sync::Arc;
use vrp_core::construction::heuristics::*;
use vrp_core::models::common::{IdDimension, ValueDimension};
use vrp_core::models::problem::{Fleet, Single};

const VIOLATION_CODE: i32 = 1;
const STATE_KEY: i32 = 2;

fn create_test_fleet() -> Fleet {
    Fleet::new(
        vec![Arc::new(test_driver())],
        vec![Arc::new(test_vehicle("v1")), Arc::new(test_vehicle("v2"))],
        Box::new(|actors| create_typed_actor_groups(actors)),
    )
}

fn create_test_single(group: Option<&str>) -> Arc<Single> {
    let mut single = create_single_with_location(Some(DEFAULT_JOB_LOCATION));
    if let Some(group) = group {
        single.dimens.set_value("group", group.to_string())
    }

    Arc::new(single)
}

fn create_test_solution_context(
    fleet: &Fleet,
    routes: Vec<(&str, Vec<Option<&str>>)>,
    actor_groups: Option<Vec<(&str, &str)>>,
) -> SolutionContext {
    let state: HashMap<_, StateValue> = if let Some(actor_groups) = actor_groups {
        let mut state: HashMap<_, StateValue> = HashMap::default();
        state.insert(
            STATE_KEY,
            Arc::new(
                actor_groups
                    .into_iter()
                    .map(|(group, vehicle)| (group.to_string(), get_actor(fleet, vehicle)))
                    .collect::<HashMap<_, _>>(),
            ),
        );
        state
    } else {
        HashMap::default()
    };

    SolutionContext {
        routes: routes
            .into_iter()
            .map(|(vehicle, groups)| {
                RouteContext::new_with_state(
                    Arc::new(create_route_with_activities(
                        &fleet,
                        vehicle,
                        groups
                            .into_iter()
                            .map(|group| create_activity_with_job_at_location(create_test_single(group), 1))
                            .collect(),
                    )),
                    Arc::new(RouteState::default()),
                )
            })
            .collect(),
        state,
        ..create_solution_context_for_fleet(fleet)
    }
}

fn get_actor(fleet: &Fleet, vehicle: &str) -> Arc<Actor> {
    fleet.actors.iter().find(|actor| actor.vehicle.dimens.get_id().unwrap() == vehicle).unwrap().clone()
}

fn compare_actor_groups(fleet: &Fleet, original: &HashMap<String, Arc<Actor>>, expected: Vec<(&str, &str)>) {
    let test = expected
        .iter()
        .map(|(group, vehicle)| (group.to_string(), get_actor(fleet, vehicle)))
        .collect::<HashMap<_, _>>();

    assert_eq!(original.len(), test.len());
    assert!(original.keys().all(|k| test[k] == original[k]));
}

#[test]
fn can_build_expected_module() {
    let module = GroupModule::new(VIOLATION_CODE, STATE_KEY);

    assert_eq!(module.state_keys().cloned().collect::<Vec<_>>(), vec![STATE_KEY]);
    assert_eq!(module.get_constraints().count(), 1);
}

parameterized_test! {can_accept_insertion, (routes, job_group, actor_groups, expected), {
    can_accept_insertion_impl(routes, job_group, actor_groups, expected);
}}

can_accept_insertion! {
    case_01: (vec![("v1", vec![None])], Some("g1"), None, vec![("g1", "v1")]),
    case_02: (vec![("v1", vec![None])], Some("g1"), Some(vec![("g2", "v2")]), vec![("g1", "v1"), ("g2", "v2")]),
}

fn can_accept_insertion_impl(
    routes: Vec<(&str, Vec<Option<&str>>)>,
    job_group: Option<&str>,
    actor_groups: Option<Vec<(&str, &str)>>,
    expected: Vec<(&str, &str)>,
) {
    let fleet = create_test_fleet();
    let module = GroupModule::new(VIOLATION_CODE, STATE_KEY);
    let mut solution = create_test_solution_context(&fleet, routes, actor_groups);
    let job = Job::Single(create_test_single(job_group));

    module.accept_insertion(&mut solution, 0, &job);

    compare_actor_groups(&fleet, get_actor_groups(&mut solution, STATE_KEY).unwrap(), expected);
}

parameterized_test! {can_accept_solution_state, (routes, actor_groups, expected), {
    can_accept_solution_state_impl(routes, actor_groups, expected);
}}

can_accept_solution_state! {
    case_01: (vec![("v1", vec![Some("g1")])], None, vec![("g1", "v1")]),
    case_02: (vec![("v1", vec![Some("g1")]), ("v2", vec![Some("g2")])], None, vec![("g1", "v1"), ("g2", "v2")]),
    case_03: (vec![("v1", vec![Some("g1")]), ("v1", vec![Some("g2")])], None, vec![("g1", "v1"), ("g2", "v1")]),
    case_04: (vec![("v1", vec![Some("g1")])], Some(vec![("g2", "v2")]), vec![("g1", "v1")]),
    case_05: (vec![("v1", vec![None])], Some(vec![("g1", "v1")]), vec![]),
}

fn can_accept_solution_state_impl(
    routes: Vec<(&str, Vec<Option<&str>>)>,
    actor_groups: Option<Vec<(&str, &str)>>,
    expected: Vec<(&str, &str)>,
) {
    let fleet = create_test_fleet();
    let module = GroupModule::new(VIOLATION_CODE, STATE_KEY);
    let mut solution = create_test_solution_context(&fleet, routes, actor_groups);

    module.accept_solution_state(&mut solution);

    compare_actor_groups(&fleet, get_actor_groups(&mut solution, STATE_KEY).unwrap(), expected);
}

parameterized_test! {can_evaluate_job, (routes, route_idx, job_group, actor_groups, expected), {
    can_evaluate_job_impl(routes, route_idx, job_group, actor_groups, expected);
}}

can_evaluate_job! {
    case_01: (vec![("v1", vec![]), ("v2", vec![])], 0, Some("g1"), Some(vec![("g1", "v2")]), Some(VIOLATION_CODE)),
    case_02: (vec![("v1", vec![]), ("v2", vec![])], 0, None, Some(vec![("g1", "v2")]), None),
    case_03: (vec![("v1", vec![]), ("v2", vec![])], 1, Some("g1"), Some(vec![("g1", "v2")]), None),
    case_04: (vec![("v1", vec![])], 0, Some("g1"), None, None),
}

fn can_evaluate_job_impl(
    routes: Vec<(&str, Vec<Option<&str>>)>,
    route_idx: usize,
    job_group: Option<&str>,
    actor_groups: Option<Vec<(&str, &str)>>,
    expected: Option<i32>,
) {
    let fleet = create_test_fleet();
    let solution_ctx = create_test_solution_context(&fleet, routes, actor_groups);
    let route_ctx = solution_ctx.routes.get(route_idx).unwrap();
    let job = Job::Single(create_test_single(job_group));

    let result = GroupHardRouteConstraint { code: VIOLATION_CODE, state_key: STATE_KEY }.evaluate_job(
        &solution_ctx,
        route_ctx,
        &job,
    );

    assert_eq!(result, expected.map(|code| RouteConstraintViolation { code }));
}