use iwm_runtime_model::{PathPointResource, PathResource};

use crate::{helpers::as_number, RuntimeInstance, RuntimeValue};

#[derive(Debug, Clone, Copy)]
struct ControlNode {
    point: PathPointResource,
    distance: f64,
}

pub(crate) fn start_path(
    instance: &mut RuntimeInstance,
    path: &PathResource,
    speed: f64,
    end_action: i32,
    absolute: bool,
) {
    let nodes = control_nodes(path);
    let Some(start) = nodes.first().copied() else {
        stop_path(instance);
        return;
    };
    let end = nodes.last().copied().unwrap_or(start);
    if end.distance <= 0.0 {
        stop_path(instance);
        return;
    }

    let forwards = speed >= 0.0;
    let point = if forwards { start.point } else { end.point };
    if absolute {
        instance.x = point.x;
        instance.y = point.y;
    }
    set_number(instance, "path_index", path.id as f64);
    set_number(instance, "path_speed", speed);
    set_number(instance, "path_endaction", end_action as f64);
    set_number(instance, "path_position", if forwards { 0.0 } else { 1.0 });
    set_number(
        instance,
        "path_positionprevious",
        if forwards { 0.0 } else { 1.0 },
    );
    set_number(instance, "path_scale", 1.0);
    set_number(instance, "path_orientation", 0.0);
    set_number(instance, "path_xstart", instance.x);
    set_number(instance, "path_ystart", instance.y);
}

pub(crate) fn advance_path(instance: &mut RuntimeInstance, paths: &[PathResource]) -> bool {
    let Some(path_id) = number(instance, "path_index")
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value.round() as usize)
    else {
        return false;
    };
    let Some(path) = paths.iter().find(|path| path.id == path_id) else {
        stop_path(instance);
        return false;
    };
    let nodes = control_nodes(path);
    let Some(first) = nodes.first().copied() else {
        stop_path(instance);
        return false;
    };
    let last = nodes.last().copied().unwrap_or(first);
    if last.distance <= 0.0 {
        stop_path(instance);
        return false;
    }

    let speed = number(instance, "path_speed").unwrap_or(0.0);
    let scale = number(instance, "path_scale").unwrap_or(1.0);
    if speed == 0.0 || scale == 0.0 {
        return false;
    }
    let previous = number(instance, "path_position").unwrap_or(0.0);
    set_number(instance, "path_positionprevious", previous.clamp(0.0, 1.0));
    let point_speed = point_at(&nodes, last.distance, previous).speed;
    let mut position = previous + speed * (point_speed / 100.0) / (last.distance * scale);
    let end_action = number(instance, "path_endaction").unwrap_or(0.0).round() as i32;
    if position <= 0.0 || position >= 1.0 {
        let reversed = position < 0.0;
        let opposite = if reversed {
            position + 1.0
        } else {
            position - 1.0
        };
        match end_action {
            1 => position = opposite,
            2 => {
                position = opposite;
                let start = if reversed { last.point } else { first.point };
                let end = if reversed { first.point } else { last.point };
                let (dx, dy) = rotate(
                    end.x - start.x,
                    end.y - start.y,
                    number(instance, "path_orientation").unwrap_or(0.0),
                );
                set_number(
                    instance,
                    "path_xstart",
                    number(instance, "path_xstart").unwrap_or(instance.x) + dx * scale,
                );
                set_number(
                    instance,
                    "path_ystart",
                    number(instance, "path_ystart").unwrap_or(instance.y) + dy * scale,
                );
            }
            3 => {
                position = 1.0 - opposite;
                set_number(
                    instance,
                    "path_speed",
                    if reversed { speed.abs() } else { -speed.abs() },
                );
            }
            _ => {
                position = 1.0;
                set_number(instance, "path_index", -1.0);
            }
        }
    }

    set_number(instance, "path_position", position);
    let point = point_at(&nodes, last.distance, position);
    let (dx, dy) = rotate(
        (point.x - first.point.x) * scale,
        (point.y - first.point.y) * scale,
        number(instance, "path_orientation").unwrap_or(0.0),
    );
    let new_x = number(instance, "path_xstart").unwrap_or(instance.x) + dx;
    let new_y = number(instance, "path_ystart").unwrap_or(instance.y) + dy;
    let direction = (instance.y - new_y).atan2(new_x - instance.x).to_degrees();
    instance.set_direction(direction);
    instance.set_speed(0.0);
    instance.x = new_x;
    instance.y = new_y;
    true
}

fn stop_path(instance: &mut RuntimeInstance) {
    set_number(instance, "path_index", -1.0);
}

fn number(instance: &RuntimeInstance, key: &str) -> Option<f64> {
    instance.vars.get(key).and_then(as_number)
}

fn set_number(instance: &mut RuntimeInstance, key: &str, value: f64) {
    instance
        .vars
        .insert(key.into(), RuntimeValue::Number(value));
}

fn rotate(x: f64, y: f64, degrees: f64) -> (f64, f64) {
    let radians = degrees.to_radians();
    (
        x * radians.cos() - y * radians.sin(),
        x * radians.sin() + y * radians.cos(),
    )
}

fn control_nodes(path: &PathResource) -> Vec<ControlNode> {
    let mut nodes = Vec::new();
    if path.smooth {
        if let (Some(&first), Some(&last)) = (path.points.first(), path.points.last()) {
            if !path.closed {
                push_node(&mut nodes, first);
            }
            let count = if path.closed {
                path.points.len()
            } else {
                path.points.len().saturating_sub(2)
            };
            for index in 0..count {
                let point0 = path.points[index % path.points.len()];
                let point1 = path.points[(index + 1) % path.points.len()];
                let point2 = path.points[(index + 2) % path.points.len()];
                generate_smooth(
                    &mut nodes,
                    path.precision,
                    halfway(point0, point1),
                    point1,
                    halfway(point1, point2),
                );
            }
            if path.closed {
                let first_node = nodes.first().map(|node| node.point).unwrap_or(first);
                push_node(&mut nodes, first_node);
            } else {
                push_node(&mut nodes, last);
            }
        }
    } else {
        for &point in &path.points {
            push_node(&mut nodes, point);
        }
        if path.closed {
            if let Some(&first) = path.points.first() {
                push_node(&mut nodes, first);
            }
        }
    }
    nodes
}

fn generate_smooth(
    nodes: &mut Vec<ControlNode>,
    precision: u32,
    point1: PathPointResource,
    point2: PathPointResource,
    point3: PathPointResource,
) {
    if precision == 0 {
        return;
    }
    let average = PathPointResource {
        x: (point1.x + point2.x * 2.0 + point3.x) / 4.0,
        y: (point1.y + point2.y * 2.0 + point3.y) / 4.0,
        speed: (point1.speed + point2.speed * 2.0 + point3.speed) / 4.0,
    };
    if distance(point1, point2) > 4.0 {
        generate_smooth(
            nodes,
            precision - 1,
            point1,
            halfway(point1, point2),
            average,
        );
    }
    push_node(nodes, average);
    if distance(point2, point3) > 4.0 {
        generate_smooth(
            nodes,
            precision - 1,
            average,
            halfway(point2, point3),
            point3,
        );
    }
}

fn push_node(nodes: &mut Vec<ControlNode>, point: PathPointResource) {
    let distance = nodes
        .last()
        .map(|node| node.distance + distance(point, node.point))
        .unwrap_or(0.0);
    nodes.push(ControlNode { point, distance });
}

fn point_at(nodes: &[ControlNode], length: f64, position: f64) -> PathPointResource {
    let distance = position * length;
    let first = nodes[0];
    if distance <= 0.0 {
        return first.point;
    }
    for pair in nodes.windows(2) {
        if distance >= pair[0].distance && distance <= pair[1].distance {
            let span = pair[1].distance - pair[0].distance;
            let amount = if span == 0.0 {
                0.0
            } else {
                (distance - pair[0].distance) / span
            };
            return PathPointResource {
                x: lerp(pair[0].point.x, pair[1].point.x, amount),
                y: lerp(pair[0].point.y, pair[1].point.y, amount),
                speed: lerp(pair[0].point.speed, pair[1].point.speed, amount),
            };
        }
    }
    nodes.last().map(|node| node.point).unwrap_or(first.point)
}

fn lerp(start: f64, end: f64, amount: f64) -> f64 {
    (end - start) * amount + start
}

fn halfway(left: PathPointResource, right: PathPointResource) -> PathPointResource {
    PathPointResource {
        x: (left.x + right.x) / 2.0,
        y: (left.y + right.y) / 2.0,
        speed: (left.speed + right.speed) / 2.0,
    }
}

fn distance(left: PathPointResource, right: PathPointResource) -> f64 {
    ((right.x - left.x).powi(2) + (right.y - left.y).powi(2)).sqrt()
}
