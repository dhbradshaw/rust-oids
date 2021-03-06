use frontend::input;
use app::constants::*;
use core::geometry::*;
use core::clock::Seconds;
use core::view::ViewTransform;
use core::view::WorldTransform;
use app::constants::DEAD_ZONE;
use super::events::VectorDirection;
use super::events::Event;

pub struct DefaultController {}

pub trait InputController {
	fn update<V, W, I>(input_state: &I, view_transform: &V, world_transform: &W, dt: Seconds) -> Vec<Event>
		where V: ViewTransform, W: WorldTransform, I: input::InputRead;
}

impl InputController for DefaultController {
	fn update<V, W, I>(input_state: &I, view_transform: &V, world_transform: &W, dt: Seconds) -> Vec<Event>
		where V: ViewTransform, W: WorldTransform, I: input::InputRead {
		let mut events = Vec::new();

		macro_rules! on_key_held {
			[$($key:ident -> $app_event:ident),*] => (
				$(if input_state.key_pressed(input::Key::$key) { events.push(Event::$app_event); })
				*
			);
			[$($key:ident -> $app_event:ident($app_args:expr)),*] => (
				$(if input_state.key_pressed(input::Key::$key) { events.push(Event::$app_event($app_args)); })
				*
			)

		}
		macro_rules! on_key_pressed_once {
			[$($key:ident -> $app_event:ident),*] => (
				$(if input_state.key_once(input::Key::$key) { events.push(Event::$app_event); })
				*
			)
		}

		on_key_held![
			W -> CamUp(1.),
			S -> CamDown(1.),
			A -> CamLeft(1.),
			D -> CamRight(1.),
			GamepadDPadUp -> CamUp(1.),
			GamepadDPadDown -> CamDown(1.),
			GamepadDPadLeft -> CamLeft(1.),
			GamepadDPadRight -> CamRight(1.)
		];

		on_key_pressed_once![
			F5 -> Reload,
			F1 -> ToggleGui,
			GamepadL3 -> ToggleGui,
			N0 -> CamReset,
			Home -> CamReset,
			KpHome -> CamReset,
			F6 -> SaveGenePoolToFile,
			F7 -> SaveWorldToFile,
			F8 -> RestartFromCheckpoint,
			F10 -> ToggleDebug,
			GamepadStart -> ToggleDebug,
			Z -> DeselectAll,
			L -> NextLight,
			B -> NextBackground,
			K -> PrevLight,
			V -> PrevBackground,
			G -> PrevSpeedFactor,
			GamepadL1 -> PrevSpeedFactor,
			H -> NextSpeedFactor,
			GamepadR1 -> NextSpeedFactor,
			P -> TogglePause,
			Esc -> AppQuit
		];

		let mouse_window_pos = input_state.mouse_position();
		let mouse_view_pos = view_transform.to_view(mouse_window_pos);
		let mouse_world_pos = world_transform.to_world(mouse_view_pos);

		let mouse_left_pressed = input_state.key_pressed(input::Key::MouseLeft) && !input_state.any_ctrl_pressed();
		if input_state.key_once(input::Key::MouseLeft) && input_state.any_ctrl_pressed() {
			events.push(Event::PickMinion(mouse_world_pos));
		};

		let firerate = input_state.gamepad_axis(0, input::Axis::L2);
		let firepower = input_state.gamepad_axis(0, input::Axis::R2);
		if firepower >= DEAD_ZONE {
			events.push(Event::PrimaryTrigger(firepower, firerate as f64));
		} else {
			if input_state.key_pressed(input::Key::Space) ||
				mouse_left_pressed {
				events.push(Event::PrimaryTrigger(1.0, 1.0));
			}
		}
		let thrust = Position {
			x: if input_state.key_pressed(input::Key::Right) {
				1.
			} else if input_state.key_pressed(input::Key::Left) {
				-1.
			} else {
				input_state.gamepad_axis(0, input::Axis::LStickX)
			},

			y: if input_state.key_pressed(input::Key::Up) {
				1.
			} else if input_state.key_pressed(input::Key::Down) {
				-1.
			} else {
				input_state.gamepad_axis(0, input::Axis::LStickY)
			},
		};

		let yaw = Position {
			x: input_state.gamepad_axis(0, input::Axis::RStickX),
			y: input_state.gamepad_axis(0, input::Axis::RStickY),
		};

		use cgmath::InnerSpace;
		let magnitude = thrust.magnitude2();
		events.push(Event::VectorThrust(
			if magnitude >= DEAD_ZONE {
				Some(thrust / magnitude.max(1.))
			} else {
				None
			},
			if input_state.key_pressed(input::Key::PageUp) {
				VectorDirection::Turn(TURN_SPEED)
			} else if input_state.key_pressed(input::Key::PageDown) {
				VectorDirection::Turn(-TURN_SPEED)
			} else if yaw.magnitude() >= DEAD_ZONE {
				VectorDirection::Orientation(yaw)
			} else if mouse_left_pressed {
				VectorDirection::LookAt(mouse_world_pos)
			} else if thrust.magnitude2() > 0.1 {
				VectorDirection::FromVelocity
			} else {
				VectorDirection::None
			}));
		if input_state.key_once(input::Key::MouseMiddle) {
			if input_state.any_ctrl_pressed() {
				events.push(Event::RandomizeMinion(mouse_world_pos));
			} else {
				events.push(Event::NewMinion(mouse_world_pos));
			}
		}

		match input_state.dragging() {
			input::Dragging::Begin(_, from) => {
				let from = world_transform.to_world(from);
				events.push(Event::BeginDrag(from, from));
			}
			input::Dragging::Dragging(_, from, to) => {
				events.push(Event::Drag(world_transform.to_world(from), world_transform.to_world(to)));
			}
			input::Dragging::End(_, from, to, prev) => {
				let mouse_vel = (view_transform.to_view(prev) - to) / dt.into();
				events.push(Event::EndDrag(
					world_transform.to_world(from),
					world_transform.to_world(to),
					mouse_vel,
				));
			}
			_ => {}
		}
		events
	}
}
