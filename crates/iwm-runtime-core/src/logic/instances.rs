use std::collections::HashMap;

use iwm_runtime_host::RuntimeHost;

use super::assignment::assign_instance_field_or_var;
use super::context::{RuntimeEvalContext, RuntimeExecutionScope, RuntimeInstanceCreateRequest};
use super::eval::evaluate_expr;
use super::statement::{evaluate_with_diagnostics, RuntimeStatementEnvironment};
use crate::helpers::as_number;
use crate::{LoweredLogicExpr, RuntimeInstance, RuntimeValue};

pub(super) fn assign_runtime_member_reference<H: RuntimeHost>(
    target: &LoweredLogicExpr,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    instance_index: usize,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> bool {
    let LoweredLogicExpr::MemberAccess { target, member } = target else {
        return false;
    };

    if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global") {
        return false;
    }

    if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "self") {
        assign_instance_field_or_var(
            member.clone(),
            value,
            instance,
            env.sprites,
            env.sprite_index,
        );
        return true;
    }

    if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "other") {
        let Some(context) = eval_context else {
            return false;
        };
        let Some(other) = context.other_instance() else {
            return false;
        };
        return assign_runtime_member_by_ref(
            other.instance_id as f64,
            member,
            value,
            instance,
            instance_index,
            eval_context,
            env,
        );
    }

    if let Some((target_index, _)) = object_member_assignment_target(target, scope, eval_context) {
        return assign_runtime_member_by_index(
            target_index,
            member,
            value,
            instance,
            instance_index,
            eval_context,
            env,
        );
    }

    if assign_pending_create_member_by_object_target(
        target,
        member,
        value.clone(),
        scope,
        eval_context,
        env.room_instance_creates,
    ) {
        return true;
    }

    let Some(RuntimeValue::Number(instance_ref)) = evaluate_with_diagnostics(
        target,
        Some(instance),
        Some(scope),
        eval_context,
        env,
        instance,
    ) else {
        return false;
    };

    assign_runtime_member_by_ref(
        instance_ref,
        member,
        value,
        instance,
        instance_index,
        eval_context,
        env,
    )
}

pub(super) fn pending_create_member_value(
    creates: &[RuntimeInstanceCreateRequest],
    instance_ref: f64,
    member: &str,
) -> Option<RuntimeValue> {
    creates
        .iter()
        .find(|create| create_request_ref_matches(instance_ref, create))
        .and_then(|create| create_member_value(create, member))
}

pub(super) fn pending_create_member_value_by_object_target(
    creates: &[RuntimeInstanceCreateRequest],
    target: &LoweredLogicExpr,
    member: &str,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let LoweredLogicExpr::Identifier(name) = target else {
        return None;
    };
    if name == "global" || scope.map(|scope| scope.is_local_key(name)).unwrap_or(false) {
        return None;
    }
    let object_ids = eval_context.and_then(|context| {
        context
            .place_target_ids_by_name
            .get(&name.to_ascii_lowercase())
    })?;
    creates
        .iter()
        .find(|create| object_ids.contains(&create.object_id))
        .and_then(|create| create_member_value(create, member))
}

pub(super) fn runtime_instance_create_request(
    args: &[LoweredLogicExpr],
    instance: &RuntimeInstance,
    globals: &HashMap<String, RuntimeValue>,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    pending_create_count: usize,
) -> Option<RuntimeInstanceCreateRequest> {
    let context = eval_context?;
    let x = args
        .first()
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, Some(scope), eval_context))
        .and_then(|value| as_number(&value))
        .unwrap_or(0.0);
    let y = args
        .get(1)
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, Some(scope), eval_context))
        .and_then(|value| as_number(&value))
        .unwrap_or(0.0);
    let object_id = args.get(2).and_then(|arg| {
        runtime_instance_create_object_id(arg, instance, globals, scope, context)
    })?;
    let runtime_id = context
        .room_instances
        .len()
        .saturating_add(pending_create_count);
    let instance_id = -1 - runtime_id as i32;
    Some(RuntimeInstanceCreateRequest {
        object_id,
        runtime_id,
        instance_id,
        x,
        y,
        post_create_vars: HashMap::new(),
    })
}

fn object_member_assignment_target<'a>(
    target: &LoweredLogicExpr,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&'a RuntimeEvalContext<'_>>,
) -> Option<(usize, &'a RuntimeInstance)> {
    let LoweredLogicExpr::Identifier(name) = target else {
        return None;
    };
    if scope.is_local_key(name) {
        return None;
    }
    let context = eval_context?;
    let target_object_ids = context
        .place_target_ids_by_name
        .get(&name.to_ascii_lowercase())?;
    context
        .room_instances_matching_object_ids(target_object_ids)
        .find(|(_, candidate)| candidate.alive)
}

fn assign_runtime_member_by_ref<H: RuntimeHost>(
    instance_ref: f64,
    member: &str,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    instance_index: usize,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> bool {
    if assign_pending_create_member(
        env.room_instance_creates,
        instance_ref,
        member,
        value.clone(),
    ) {
        return true;
    }

    let Some(context) = eval_context else {
        return false;
    };
    let Some((target_index, _)) = context
        .room_instances_iter()
        .find(|(_, candidate)| runtime_instance_ref_matches(instance_ref, candidate))
    else {
        return false;
    };

    assign_runtime_member_by_index(
        target_index,
        member,
        value,
        instance,
        instance_index,
        Some(context),
        env,
    )
}

fn assign_runtime_member_by_index<H: RuntimeHost>(
    target_index: usize,
    member: &str,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    instance_index: usize,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> bool {
    if target_index == instance_index {
        assign_instance_field_or_var(
            member.to_string(),
            value,
            instance,
            env.sprites,
            env.sprite_index,
        );
        return true;
    }

    let Some(mut target_instance) = env
        .room_instance_updates
        .get(target_index)
        .cloned()
        .or_else(|| eval_context.and_then(|context| context.room_instance(target_index).cloned()))
    else {
        return false;
    };
    assign_instance_field_or_var(
        member.to_string(),
        value,
        &mut target_instance,
        env.sprites,
        env.sprite_index,
    );
    env.room_instance_updates.set(target_index, target_instance);
    true
}

fn assign_pending_create_member(
    creates: &mut [RuntimeInstanceCreateRequest],
    instance_ref: f64,
    member: &str,
    value: RuntimeValue,
) -> bool {
    let Some(create) = creates
        .iter_mut()
        .find(|create| create_request_ref_matches(instance_ref, create))
    else {
        return false;
    };
    create.post_create_vars.insert(member.to_string(), value);
    true
}

fn assign_pending_create_member_by_object_target(
    target: &LoweredLogicExpr,
    member: &str,
    value: RuntimeValue,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    creates: &mut [RuntimeInstanceCreateRequest],
) -> bool {
    let LoweredLogicExpr::Identifier(name) = target else {
        return false;
    };
    if scope.is_local_key(name) {
        return false;
    }
    let Some(object_ids) = eval_context.and_then(|context| {
        context
            .place_target_ids_by_name
            .get(&name.to_ascii_lowercase())
    }) else {
        return false;
    };
    let Some(create) = creates
        .iter_mut()
        .find(|create| object_ids.contains(&create.object_id))
    else {
        return false;
    };
    create.post_create_vars.insert(member.to_string(), value);
    true
}

fn create_member_value(
    create: &RuntimeInstanceCreateRequest,
    member: &str,
) -> Option<RuntimeValue> {
    create
        .post_create_vars
        .get(member)
        .cloned()
        .or_else(|| match member {
            "x" => Some(RuntimeValue::Number(create.x)),
            "y" => Some(RuntimeValue::Number(create.y)),
            _ => None,
        })
}

fn create_request_ref_matches(instance_ref: f64, create: &RuntimeInstanceCreateRequest) -> bool {
    if !instance_ref.is_finite() {
        return false;
    }
    let rounded = instance_ref.round();
    create.instance_id as f64 == rounded || create.runtime_id as f64 == rounded
}

fn runtime_instance_ref_matches(instance_ref: f64, instance: &RuntimeInstance) -> bool {
    if !instance_ref.is_finite() {
        return false;
    }
    let rounded = instance_ref.round();
    instance.instance_id as f64 == rounded || instance.runtime_id as f64 == rounded
}

fn runtime_instance_create_object_id(
    expr: &LoweredLogicExpr,
    instance: &RuntimeInstance,
    globals: &HashMap<String, RuntimeValue>,
    scope: &RuntimeExecutionScope,
    context: &RuntimeEvalContext<'_>,
) -> Option<usize> {
    if let LoweredLogicExpr::Identifier(name) = expr {
        if let Some(object_id) = context
            .place_target_ids_by_name
            .get(&name.to_ascii_lowercase())
            .and_then(|ids| ids.first().copied())
        {
            return Some(object_id);
        }
    }

    evaluate_expr(expr, Some(instance), globals, Some(scope), Some(context))
        .and_then(|value| as_number(&value))
        .and_then(non_negative_integer_usize)
}

fn non_negative_integer_usize(value: f64) -> Option<usize> {
    if value.is_finite() && value >= 0.0 && value.fract() == 0.0 {
        Some(value as usize)
    } else {
        None
    }
}
